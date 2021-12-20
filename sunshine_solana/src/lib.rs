use solana_sdk::signature::Signature;
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

use tokio::time::Duration;
use uuid::Uuid;

mod commands;
mod error;
use error::Error;

type FlowId = GraphId;

type CommandResult = Result<(u64, Vec<Instruction>), Error>;

const START_NODE_MARKER: &str = "START_NODE_MARKER";
const COMMAND_MARKER: &str = "COMMAND_MARKER";
const INPUT_ARG_NAME_MARKER: &str = "INPUT_ARG_NAME_MARKER";
const OUTPUT_ARG_NAME_MARKER: &str = "OUTPUT_ARG_NAME_MARKER";
const CTX_EDGE_MARKER: &str = "CTX_EDGE_MARKER";
const CTX_MARKER: &str = "CTX_MARKER";

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
                println!("tick");

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
                            inputs.insert(name, rx.recv().await.unwrap());
                        }

                        let mut outputs = run_command(&node.cmd, inputs).await.unwrap();
                        assert!(outputs.len() >= node.outputs.len());

                        for (name, txs) in node.outputs.into_iter() {
                            let val = outputs.remove(&name).unwrap();
                            for tx in txs {
                                tx.send(val.clone()).unwrap();
                            }
                        }
                    });
                }

                for node in start_nodes {
                    node.send(ValueType::Empty).unwrap();
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

#[derive(Debug, Clone)]
pub enum ValueType {
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

#[derive(Debug)]
pub struct WrappedKeypair(pub Keypair);

impl From<Keypair> for WrappedKeypair {
    fn from(keypair: Keypair) -> Self {
        Self(keypair)
    }
}

impl From<WrappedKeypair> for Keypair {
    fn from(wk: WrappedKeypair) -> Keypair {
        wk.0
    }
}

impl Clone for WrappedKeypair {
    fn clone(&self) -> Self {
        let keypair = Keypair::from_bytes(&self.0.to_bytes()).unwrap();
        Self(keypair)
    }
}

pub struct Flow {
    start_nodes: Vec<Sender<ValueType>>,
    nodes: HashMap<NodeId, FlowNode>,
}

struct FlowNode {
    inputs: HashMap<String, Receiver<ValueType>>,
    outputs: HashMap<String, Vec<Sender<ValueType>>>,
    cmd: Command,
}

async fn run_command(
    cmd: &Command,
    inputs: HashMap<String, ValueType>,
) -> Result<HashMap<String, ValueType>, Error> {
    match cmd {
        Command::Simple(simple) => simple.run(inputs).await,
        Command::Solana(solana) => solana.run(inputs).await,
    }
}
