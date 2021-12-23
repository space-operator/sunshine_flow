use serde::{Deserialize, Serialize};
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use std::str::FromStr;
use std::sync::Arc;

use commands::Command;
use dashmap::DashMap;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::keypair::Keypair;
use std::collections::HashMap;
use sunshine_core::msg::{Action, GraphId, NodeId, QueryKind};
use sunshine_core::store::Datastore;

use tokio::sync::mpsc::{self, UnboundedReceiver as Receiver, UnboundedSender as Sender};
use tokio::sync::oneshot;

use parse_display::Display as ParseDisplay;
use tokio::time::Duration;
use uuid::Uuid;

pub mod commands;
mod error;
use error::Error;

type FlowId = GraphId;

type CommandResult = Result<(u64, Vec<Instruction>), Error>;

pub const START_NODE_MARKER: &str = "START_NODE_MARKER";
pub const COMMAND_MARKER: &str = "COMMAND_MARKER";
pub const INPUT_ARG_NAME_MARKER: &str = "INPUT_ARG_NAME_MARKER";
pub const OUTPUT_ARG_NAME_MARKER: &str = "OUTPUT_ARG_NAME_MARKER";
pub const CTX_EDGE_MARKER: &str = "CTX_EDGE_MARKER";
pub const CTX_MARKER: &str = "CTX_MARKER";

pub struct FlowContext {
    deployed: DashMap<FlowId, oneshot::Sender<()>>,
    db: Arc<dyn Datastore>,
}

impl FlowContext {
    pub fn new(db: Arc<dyn Datastore>) -> FlowContext {
        FlowContext {
            deployed: DashMap::new(),
            db,
        }
    }

    pub fn undeploy_flow(&self, flow_id: FlowId) -> Result<(), Error> {
        let (_, stop_signal) = self
            .deployed
            .remove(&flow_id)
            .ok_or(Error::FlowDoesntExist)?;
        stop_signal.send(()).unwrap();
        Ok(())
    }

    async fn read_flow(db: Arc<dyn Datastore>, flow_id: FlowId) -> Result<Flow, Error> {
        let graph = db
            .execute(Action::Query(QueryKind::ReadGraph(flow_id)))
            .await?
            .into_graph()
            .unwrap();

        let mut contexts = HashMap::new();

        for node in graph.nodes.iter() {
            if let Some(cfg) = node.properties.get(CTX_MARKER) {
                let cfg: commands::solana::Config = serde_json::from_value(cfg.clone()).unwrap();

                let ctx = Arc::new(commands::solana::Ctx::new(cfg, db.clone())?);

                contexts.insert(node.node_id, ctx);
            }
        }

        let mut nodes = HashMap::new();

        for node in graph.nodes.iter() {
            let cfg = match node.properties.get(COMMAND_MARKER) {
                Some(cfg) => cfg.clone(),
                None => continue,
            };

            let cfg: commands::Config = serde_json::from_value(cfg).unwrap();

            let cmd = match cfg {
                commands::Config::Simple(simple) => Command::Simple(simple),
                commands::Config::Solana(kind) => {
                    let mut ctx = None;
                    for edge in node.inbound_edges.iter() {
                        let props = db.read_edge_properties(*edge).await?;
                        if props.get(CTX_EDGE_MARKER).is_some() {
                            ctx = Some(contexts.get(&edge.from).unwrap().clone());

                            break;
                        }
                    }

                    let ctx = ctx.ok_or(Error::NoContextForCommand)?;

                    Command::Solana(commands::solana::Command { ctx, kind })
                }
            };

            nodes.insert(
                node.node_id,
                FlowNode {
                    inputs: HashMap::new(),
                    outputs: HashMap::new(),
                    cmd,
                },
            );
        }

        let mut start_nodes = Vec::new();

        for node in graph.nodes.iter() {
            if node.properties.get(COMMAND_MARKER).is_none() {
                continue;
            }

            for edge in node.outbound_edges.iter() {
                let properties = db
                    .execute(Action::Query(QueryKind::ReadEdgeProperties(*edge)))
                    .await
                    .unwrap()
                    .into_properties()
                    .unwrap();

                let input_arg_name = properties
                    .get(INPUT_ARG_NAME_MARKER)
                    .unwrap()
                    .as_str()
                    .unwrap();

                let output_arg_name = properties
                    .get(OUTPUT_ARG_NAME_MARKER)
                    .unwrap()
                    .as_str()
                    .unwrap();

                let (tx, rx) = mpsc::unbounded_channel();

                use std::collections::hash_map::Entry;

                let outputs = &mut nodes.get_mut(&edge.from).unwrap().outputs;

                match outputs.entry(output_arg_name.to_owned()) {
                    Entry::Occupied(mut entry) => entry.get_mut().push(tx),
                    Entry::Vacant(entry) => {
                        entry.insert(vec![tx]);
                    }
                }

                nodes
                    .get_mut(&edge.to)
                    .unwrap()
                    .inputs
                    .insert(input_arg_name.to_owned(), rx);
            }
            if node.properties.contains_key(START_NODE_MARKER) {
                let (tx, rx) = mpsc::unbounded_channel();
                nodes
                    .get_mut(&node.node_id)
                    .unwrap()
                    .inputs
                    .insert("STARTER_INPUT_MARKER".into(), rx);
                start_nodes.push(tx);
            }
        }

        Ok(Flow { start_nodes, nodes })
    }

    pub async fn deploy_flow(&self, period: Duration, flow_id: FlowId) -> Result<(), Error> {
        let mut interval = tokio::time::interval(period);

        let db = self.db.clone();

        let interval_fut = async move {
            loop {
                interval.tick().await;

                let Flow { nodes, start_nodes } = match Self::read_flow(db.clone(), flow_id).await {
                    Ok(flow) => flow,
                    Err(e) => {
                        eprintln!("failed to read flow: {}", e);
                        return;
                    }
                };

                for (_, node) in nodes {
                    tokio::spawn(async move {
                        let mut inputs = HashMap::new();
                        for (name, mut rx) in node.inputs {
                            let input = match rx.recv().await {
                                Some(input) => input,
                                None => {
                                    eprintln!("can't receive input quitting");
                                    return;
                                }
                            };
                            inputs.insert(name, input);
                        }

                        println!("executing {:?}", node.cmd.kind());
                        println!("{:#?}", &inputs);

                        let mut outputs = match run_command(&node.cmd, inputs.clone()).await {
                            Ok(outputs) => outputs,
                            Err(e) => {
                                eprintln!("failed to run command {}", e);
                                return;
                            }
                        };

                        for (name, value) in inputs {
                            if !outputs.contains_key(&name) {
                                outputs.insert(name, value);
                            }
                        }

                        println!("executed {:?}", node.cmd.kind());

                        for (name, txs) in node.outputs.into_iter() {
                            let val = match outputs.get(&name) {
                                Some(val) => val.clone(),
                                None => {
                                    eprintln!("output with name {} not found", name);
                                    return;
                                }
                            };
                            for tx in txs {
                                tx.send(val.clone()).unwrap();
                            }
                        }
                    });
                }

                for node in start_nodes {
                    node.send(Value::Empty).unwrap();
                }
            }
        };

        let (send_stop_signal, stop_signal) = oneshot::channel();

        tokio::spawn(async {
            tokio::select! {
                _ = interval_fut => (),
                _ = stop_signal => (),
            }
        });

        self.deployed.insert(flow_id, send_stop_signal);

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Integer(i64),
    Keypair(WrappedKeypair),
    String(String),
    NodeId(Uuid),
    DeletedNode(Uuid),
    Pubkey(Pubkey),
    Success(Signature),
    Balance(u64),
    U8(u8),
    U64(u64),
    F64(f64),
    Bool(bool),
    StringOpt(Option<String>),
    Empty,
    NodeIdOpt(Option<NodeId>),
}

impl Value {
    fn kind(&self) -> ValueKind {
        match self {
            Value::Integer(_) => ValueKind::Integer,
            Value::Keypair(_) => ValueKind::Keypair,
            Value::String(_) => ValueKind::String,
            Value::NodeId(_) => ValueKind::NodeId,
            Value::DeletedNode(_) => ValueKind::DeletedNode,
            Value::Pubkey(_) => ValueKind::Pubkey,
            Value::Success(_) => ValueKind::Success,
            Value::Balance(_) => ValueKind::Balance,
            Value::U8(_) => ValueKind::U8,
            Value::U64(_) => ValueKind::U64,
            Value::F64(_) => ValueKind::F64,
            Value::Bool(_) => ValueKind::Bool,
            Value::StringOpt(_) => ValueKind::StringOpt,
            Value::Empty => ValueKind::Empty,
            Value::NodeIdOpt(_) => ValueKind::NodeIdOpt,
        }
    }
}

impl TryInto<Pubkey> for Value {
    type Error = Error;

    fn try_into(self) -> Result<Pubkey, Error> {
        let res = match self {
            Value::Keypair(kp) => {
                let kp: Keypair = kp.into();
                kp.pubkey()
            }
            Value::Pubkey(p) => p,
            Value::String(s) => Pubkey::from_str(s.as_str())?,
            _ => return Err(Error::ValueIntoError(self.kind(), "Pubkey".to_owned())),
        };

        Ok(res)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ParseDisplay)]
#[display(style = "snake_case")]
pub enum ValueKind {
    Integer,
    Keypair,
    String,
    NodeId,
    DeletedNode,
    Pubkey,
    Success,
    Balance,
    U8,
    U64,
    F64,
    Bool,
    StringOpt,
    Empty,
    NodeIdOpt,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WrappedKeypair(String);

impl From<Keypair> for WrappedKeypair {
    fn from(keypair: Keypair) -> Self {
        Self(keypair.to_base58_string())
    }
}

impl From<WrappedKeypair> for Keypair {
    fn from(wk: WrappedKeypair) -> Keypair {
        Keypair::from_base58_string(&wk.0)
    }
}

pub struct Flow {
    start_nodes: Vec<Sender<Value>>,
    nodes: HashMap<NodeId, FlowNode>,
}

struct FlowNode {
    inputs: HashMap<String, Receiver<Value>>,
    outputs: HashMap<String, Vec<Sender<Value>>>,
    cmd: Command,
}

async fn run_command(
    cmd: &Command,
    inputs: HashMap<String, Value>,
) -> Result<HashMap<String, Value>, Error> {
    match cmd {
        Command::Simple(simple) => simple.run(inputs).await,
        Command::Solana(solana) => solana.run(inputs).await,
    }
}
