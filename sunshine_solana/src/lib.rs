use serde::{Deserialize, Serialize};
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use commands::Command;
use dashmap::DashMap;
use mpl_token_metadata::state::Creator;
use serde_json::Value as JsonValue;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::keypair::Keypair;
use std::collections::HashMap;
use sunshine_core::msg::{Action, CreateEdge, GraphId, NodeId, Properties, QueryKind};
use sunshine_core::store::Datastore;

use tokio::sync::mpsc::{self, UnboundedReceiver as Receiver, UnboundedSender as Sender};
use tokio::sync::watch;

use parse_display::Display as ParseDisplay;

use uuid::Uuid;

pub mod commands;
mod error;
use error::Error;

use commands::solana::nft::update_metadata_accounts::MetadataAccountData;

pub use commands::solana::Config as ContextConfig;
pub use commands::Config as CommandConfig;

type FlowId = GraphId;

type CommandResult = Result<(u64, Vec<Instruction>), Error>;

pub const START_NODE_MARKER: &str = "START_NODE_MARKER";
pub const COMMAND_MARKER: &str = "COMMAND_MARKER";
pub const INPUT_ARG_NAME_MARKER: &str = "INPUT_ARG_NAME_MARKER";
pub const OUTPUT_ARG_NAME_MARKER: &str = "OUTPUT_ARG_NAME_MARKER";
pub const CTX_EDGE_MARKER: &str = "CTX_EDGE_MARKER";
pub const CTX_MARKER: &str = "CTX_MARKER";
pub const COMMAND_NAME_MARKER: &str = "COMMAND_NAME_MARKER";

pub struct FlowContext {
    deployed: DashMap<FlowId, watch::Sender<u8>>,
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
        stop_signal.send(5).unwrap();
        Ok(())
    }

    async fn read_flow(db: Arc<dyn Datastore>, flow_id: FlowId) -> Result<Flow, Error> {
        let graph = db
            .execute(Action::Query(QueryKind::ReadGraph(flow_id)))
            .await?
            .into_graph()
            .unwrap();

        let (_, log_graph_id) = db.create_graph(Default::default()).await.unwrap();

        let timestamp = chrono::offset::Utc::now().timestamp_millis();
        let timestamp = JsonValue::Number(serde_json::Number::from(timestamp));

        let mut props = Properties::default();

        props.insert("timestamp".to_owned(), timestamp);

        db.create_edge(
            CreateEdge {
                from: flow_id,
                to: log_graph_id,
                properties: props,
            },
            flow_id,
        )
        .await
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

            let name = node
                .properties
                .get(COMMAND_NAME_MARKER)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned();

            let mut props = Properties::new();

            props.insert(
                "original_props".to_owned(),
                JsonValue::Object(node.properties.clone()),
            );

            props.insert(
                "original_node_id".to_owned(),
                JsonValue::String(node.node_id.to_string()),
            );

            let (_, log_node_id) = db.create_node((log_graph_id, props)).await.unwrap();

            nodes.insert(
                node.node_id,
                FlowNode {
                    name,
                    inputs: HashMap::new(),
                    outputs: HashMap::new(),
                    cmd,
                    log_node_id,
                },
            );
        }

        let mut start_nodes = Vec::new();

        for node in graph.nodes.iter() {
            if node.properties.get(COMMAND_MARKER).is_none() {
                continue;
            }

            for edge in node.outbound_edges.iter() {
                db.create_edge(
                    CreateEdge {
                        from: nodes.get(&edge.from).unwrap().log_node_id,
                        to: nodes.get(&edge.to).unwrap().log_node_id,
                        properties: Default::default(),
                    },
                    log_graph_id,
                )
                .await
                .unwrap();

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

                let overridden = nodes
                    .get_mut(&edge.to)
                    .unwrap()
                    .inputs
                    .insert(input_arg_name.to_owned(), rx);

                assert!(overridden.is_none());
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

        Ok(Flow {
            start_nodes,
            nodes,
            log_graph_id,
        })
    }

    pub async fn deploy_flow(&self, schedule: Schedule, flow_id: FlowId) -> Result<(), Error> {
        if self.deployed.contains_key(&flow_id) {
            return Err(Error::FlowAlreadyDeployed);
        }

        let (send_stop_signal, stop_signal) = watch::channel(1u8);

        let res = match schedule {
            Schedule::Once => {
                Self::run_flow(self.db.clone(), flow_id, stop_signal).await;
            }
            Schedule::Interval(period) => {
                self.start_flow_with_interval(period, flow_id, stop_signal)
                    .await?
            }
        };

        self.deployed.insert(flow_id, send_stop_signal);

        Ok(res)
    }

    async fn start_flow_with_interval(
        &self,
        period: Duration,
        flow_id: FlowId,
        mut stop_signal: watch::Receiver<u8>,
    ) -> Result<(), Error> {
        let mut interval = tokio::time::interval(period);

        let db = self.db.clone();

        let stop_signal_c = stop_signal.clone();

        let interval_fut = async move {
            loop {
                interval.tick().await;
                Self::run_flow(db.clone(), flow_id, stop_signal_c.clone()).await;
            }
        };

        tokio::spawn(async move {
            tokio::select! {
                _ = interval_fut => (),
                _ = stop_signal.changed() => (),
            }
        });

        Ok(())
    }

    async fn run_flow(db: Arc<dyn Datastore>, flow_id: FlowId, stop_signal: watch::Receiver<u8>) {
        let Flow {
            nodes,
            start_nodes,
            log_graph_id,
        } = match Self::read_flow(db.clone(), flow_id).await {
            Ok(flow) => flow,
            Err(e) => {
                eprintln!("failed to read flow: {}", e);
                return;
            }
        };

        for (_, node) in nodes {
            let db = db.clone();

            let mut stop_signal = stop_signal.clone();

            let cmd_fut = async move {
                let mut inputs = HashMap::new();

                let mut props = db.read_node(node.log_node_id).await.unwrap().properties;

                props.insert(
                    "kind".to_owned(),
                    JsonValue::String(format!("{:#?}", node.cmd.kind())),
                );

                props.insert("name".to_owned(), JsonValue::String(node.name.clone()));

                props.insert("success".into(), JsonValue::Bool(true));

                if let Err(e) = db
                    .update_node((node.log_node_id, props), log_graph_id)
                    .await
                {
                    eprintln!("failed to update log node for command: {}", e);
                };

                let append_log = |db: Arc<dyn Datastore>, msg: String, fail: bool| async move {
                    let time = chrono::offset::Utc::now().timestamp_millis().to_string();

                    let mut props = db.read_node(node.log_node_id).await.unwrap().properties;

                    props.insert(time, JsonValue::String(msg));

                    if fail {
                        props.insert("success".into(), JsonValue::Bool(false));
                    }

                    if let Err(e) = db
                        .update_node((node.log_node_id, props.clone()), log_graph_id)
                        .await
                    {
                        eprintln!("failed to update logs for command: {}", e);
                    }
                };

                append_log(db.clone(), "WAITING FOR INPUTS".into(), false).await;

                for (name, mut rx) in node.inputs {
                    let input = match rx.recv().await {
                        Some(input) => input,
                        None => {
                            append_log(db.clone(), "can't receive input, quitting".into(), true)
                                .await;
                            return;
                        }
                    };
                    inputs.insert(name, input);
                }

                append_log(
                    db.clone(),
                    format!("starting to execute with inputs: {:#?}", &inputs),
                    false,
                )
                .await;

                let outputs = match run_command(&node.cmd, inputs.clone()).await {
                    Ok(outputs) => outputs,
                    Err(e) => {
                        append_log(db.clone(), format!("failed to run command: {:#?}", e), true)
                            .await;
                        return;
                    }
                };

                append_log(
                    db.clone(),
                    "finished execution, writing outputs".into(),
                    false,
                )
                .await;

                for (name, txs) in node.outputs.into_iter() {
                    let val = match outputs.get(&name) {
                        Some(val) => val.clone(),
                        None => {
                            append_log(
                                db.clone(),
                                format!("output with name {} not found", name),
                                true,
                            )
                            .await;
                            return;
                        }
                    };
                    for tx in txs {
                        tx.send(val.clone()).unwrap();
                    }
                }
            };

            tokio::spawn(async move {
                tokio::select! {
                    _ = cmd_fut => (),
                    _ = stop_signal.changed() => (),
                }
            });
        }

        for node in start_nodes {
            node.send(Value::Empty).unwrap();
        }
    }
}

pub enum Schedule {
    Once,
    Interval(Duration),
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
    U16(u16),
    U64(u64),
    F32(f32),
    F64(f64),
    Bool(bool),
    StringOpt(Option<String>),
    Empty,
    NodeIdOpt(Option<NodeId>),
    NftCreators(Vec<NftCreator>),
    MetadataAccountData(MetadataAccountData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftCreator {
    pub address: Pubkey,
    pub verified: bool,
    pub share: u8,
}

impl From<NftCreator> for Creator {
    fn from(nft_creator: NftCreator) -> Creator {
        Creator {
            address: nft_creator.address,
            verified: nft_creator.verified,
            share: nft_creator.share,
        }
    }
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
            Value::U16(_) => ValueKind::U16,
            Value::U64(_) => ValueKind::U64,
            Value::F32(_) => ValueKind::F32,
            Value::F64(_) => ValueKind::F64,
            Value::Bool(_) => ValueKind::Bool,
            Value::StringOpt(_) => ValueKind::StringOpt,
            Value::Empty => ValueKind::Empty,
            Value::NodeIdOpt(_) => ValueKind::NodeIdOpt,
            Value::NftCreators(_) => ValueKind::NftCreators,
            Value::MetadataAccountData(_) => ValueKind::MetadataAccountData,
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
    U16,
    U64,
    F32,
    F64,
    Bool,
    StringOpt,
    Empty,
    NodeIdOpt,
    NftCreators,
    MetadataAccountData,
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
    log_graph_id: GraphId,
}

struct FlowNode {
    log_node_id: NodeId,
    name: String,
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
