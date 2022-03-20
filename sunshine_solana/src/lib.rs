use commands::simple::branch::Operator;
use commands::solana::nft::create_metadata_accounts::NftUses;
use serde::{Deserialize, Serialize};
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use std::fmt;
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

type RunId = Uuid;

type CommandResult = Result<(u64, Vec<Instruction>), Error>;

pub const START_NODE_MARKER: &str = "START_NODE_MARKER";
pub const COMMAND_MARKER: &str = "COMMAND_MARKER";
pub const INPUT_ARG_NAME_MARKER: &str = "INPUT_ARG_NAME_MARKER";
pub const OUTPUT_ARG_NAME_MARKER: &str = "OUTPUT_ARG_NAME_MARKER";
pub const CTX_EDGE_MARKER: &str = "CTX_EDGE_MARKER";
pub const CTX_MARKER: &str = "CTX_MARKER";
pub const COMMAND_NAME_MARKER: &str = "COMMAND_NAME_MARKER";
pub const RUN_ID_MARKER: &str = "RUN_ID_MARKER";

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
        if stop_signal.send(5).is_err() {
            eprintln!("flow already undeployed itself");
        }
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

        let run_id = Uuid::new_v4();

        props.insert("timestamp".to_owned(), timestamp);
        props.insert(
            RUN_ID_MARKER.to_owned(),
            JsonValue::String(run_id.to_string()),
        );

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

                if overridden.is_some() {
                    return Err(Error::MultipleOutputsToSameInput);
                }
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
            run_id,
        })
    }

    pub async fn deploy_flow(
        &self,
        schedule: Schedule,
        flow_id: FlowId,
    ) -> Result<Option<RunId>, Error> {
        self.undeploy_flow(flow_id).ok();

        let (send_stop_signal, stop_signal) = watch::channel(1u8);

        let res = match schedule {
            Schedule::Once => Self::run_flow(self.db.clone(), flow_id, stop_signal).await,
            Schedule::Interval(period) => {
                self.start_flow_with_interval(period, flow_id, stop_signal)
                    .await?;

                None
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

    async fn run_flow(
        db: Arc<dyn Datastore>,
        flow_id: FlowId,
        stop_signal: watch::Receiver<u8>,
    ) -> Option<RunId> {
        use std::time::Instant;

        let Flow {
            nodes,
            start_nodes,
            log_graph_id,
            run_id,
        } = match Self::read_flow(db.clone(), flow_id).await {
            Ok(flow) => flow,
            Err(e) => {
                eprintln!("failed to read flow: {}", e);
                return None;
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

                props.insert(
                    "state".to_owned(),
                    serde_json::to_value(&RunState::WaitingInputs).unwrap(),
                );

                if let Err(e) = db
                    .update_node((node.log_node_id, props), log_graph_id)
                    .await
                {
                    eprintln!("failed to update log node for command: {}", e);
                };

                let change_state = |db: Arc<dyn Datastore>, state: RunState| async move {
                    let mut props = db.read_node(node.log_node_id).await.unwrap().properties;

                    props.insert("state".to_owned(), serde_json::to_value(state).unwrap());

                    if let Err(e) = db
                        .update_node((node.log_node_id, props.clone()), log_graph_id)
                        .await
                    {
                        eprintln!("failed to update logs for command: {}", e);
                    }
                };

                for (name, mut rx) in node.inputs {
                    let input = match rx.recv().await {
                        Some(input) => match input {
                            Value::Cancel => {
                                for (_, txs) in node.outputs {
                                    for tx in txs {
                                        tx.send(Value::Cancel).ok();
                                    }
                                }
                                change_state(db.clone(), RunState::Canceled).await;
                                return;
                            }
                            v => v,
                        },
                        None => {
                            change_state(
                                db.clone(),
                                RunState::Failed(0, "can't receive input, quitting".into()),
                            )
                            .await;
                            return;
                        }
                    };
                    inputs.insert(name, input);
                }

                {
                    let mut props = db.read_node(node.log_node_id).await.unwrap().properties;

                    props.insert("inputs".to_owned(), serde_json::to_value(&inputs).unwrap());

                    if let Err(e) = db
                        .update_node((node.log_node_id, props.clone()), log_graph_id)
                        .await
                    {
                        eprintln!("failed to update logs for command: {}", e);
                    }
                }

                change_state(db.clone(), RunState::Running).await;

                let start = Instant::now();

                let outputs = match run_command(&node.cmd, inputs.clone()).await {
                    Ok(outputs) => outputs,
                    Err(e) => {
                        change_state(
                            db.clone(),
                            RunState::Failed(
                                start.elapsed().as_millis() as u64,
                                format!("failed to run command: {:#?}", e),
                            ),
                        )
                        .await;
                        return;
                    }
                };

                if let Some(output) = outputs.get("__print_output") {
                    let mut props = db.read_node(node.log_node_id).await.unwrap().properties;

                    let output = match output {
                        Value::String(output) => output,
                        _ => unreachable!(),
                    };

                    props.insert(
                        "__print_output".to_owned(),
                        JsonValue::String(output.clone()),
                    );

                    if let Err(e) = db
                        .update_node((node.log_node_id, props.clone()), log_graph_id)
                        .await
                    {
                        eprintln!("failed to update logs for command: {}", e);
                    }
                }

                let mut node_outputs = node.outputs;

                if let Some(_) = outputs.get("__true_branch") {
                    match node_outputs.remove("__true_branch") {
                        Some(txs) => {
                            for tx in txs {
                                tx.send(Value::Empty).ok();
                            }
                        }
                        None => {
                            change_state(
                                db.clone(),
                                RunState::Failed(
                                    start.elapsed().as_millis() as u64,
                                    "output with name __true_branch not found".to_owned(),
                                ),
                            )
                            .await;
                            return;
                        }
                    }
                    match node_outputs.remove("__false_branch") {
                        Some(txs) => {
                            for tx in txs {
                                tx.send(Value::Cancel).ok();
                            }
                        }
                        None => {
                            change_state(
                                db.clone(),
                                RunState::Failed(
                                    start.elapsed().as_millis() as u64,
                                    "output with name __false_branch not found".to_owned(),
                                ),
                            )
                            .await;
                            return;
                        }
                    }
                } else if let Some(_) = outputs.get("__false_branch") {
                    match node_outputs.remove("__false_branch") {
                        Some(txs) => {
                            for tx in txs {
                                tx.send(Value::Empty).ok();
                            }
                        }
                        None => {
                            change_state(
                                db.clone(),
                                RunState::Failed(
                                    start.elapsed().as_millis() as u64,
                                    "output with name __false_branch not found".to_owned(),
                                ),
                            )
                            .await;
                            return;
                        }
                    }
                    match node_outputs.remove("__true_branch") {
                        Some(txs) => {
                            for tx in txs {
                                tx.send(Value::Cancel).ok();
                            }
                        }
                        None => {
                            change_state(
                                db.clone(),
                                RunState::Failed(
                                    start.elapsed().as_millis() as u64,
                                    "output with name __true_branch not found".to_owned(),
                                ),
                            )
                            .await;
                            return;
                        }
                    }
                } else {
                    for (name, txs) in node_outputs.into_iter() {
                        let val = match outputs.get(&name) {
                            Some(val) => val.clone(),
                            None => {
                                change_state(
                                    db.clone(),
                                    RunState::Failed(
                                        start.elapsed().as_millis() as u64,
                                        format!("output with name {} not found", name),
                                    ),
                                )
                                .await;
                                return;
                            }
                        };
                        for tx in txs {
                            tx.send(val.clone()).ok();
                        }
                    }
                }

                change_state(
                    db.clone(),
                    RunState::Success(start.elapsed().as_millis() as u64),
                )
                .await;
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

        Some(run_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RunState {
    WaitingInputs,
    Running,
    Failed(u64, String),
    Success(u64),
    Canceled,
}

pub enum Schedule {
    Once,
    Interval(Duration),
}

#[derive(Debug, Clone, Serialize, Deserialize, derive_more::Display)]
pub enum Value {
    #[display(fmt = "{}", _0)]
    I64(i64),
    #[display(fmt = "{}", _0)]
    Keypair(WrappedKeypair),
    #[display(fmt = "{}", _0)]
    String(String),
    #[display(fmt = "{}", _0)]
    NodeId(Uuid),
    #[display(fmt = "{}", _0)]
    DeletedNode(Uuid),
    #[display(fmt = "{}", _0)]
    Pubkey(Pubkey),
    #[display(fmt = "{}", _0)]
    Success(Signature),
    #[display(fmt = "{}", _0)]
    Balance(u64),
    #[display(fmt = "{}", _0)]
    U8(u8),
    #[display(fmt = "{}", _0)]
    U16(u16),
    #[display(fmt = "{}", _0)]
    U64(u64),
    #[display(fmt = "{}", _0)]
    F32(f32),
    #[display(fmt = "{}", _0)]
    F64(f64),
    #[display(fmt = "{}", _0)]
    Bool(bool),
    #[display(fmt = "{:?}", _0)]
    StringOpt(Option<String>),
    #[display(fmt = "empty")]
    Empty,
    #[display(fmt = "{:?}", _0)]
    NodeIdOpt(Option<NodeId>),
    #[display(fmt = "{:?}", _0)]
    NftCreators(Vec<NftCreator>),
    #[display(fmt = "{:?}", _0)]
    MetadataAccountData(MetadataAccountData),
    #[display(fmt = "{:?}", _0)]
    Uses(NftUses),
    #[display(fmt = "{}", _0)]
    NftMetadata(NftMetadata),
    #[display(fmt = "{:?}", _0)]
    Operator(Operator),
    #[display(fmt = "{}", _0)]
    Json(JsonValueWrapper),
    #[display(fmt = "cancel")]
    Cancel,
}

impl TryFrom<JsonValue> for Value {
    type Error = Error;

    fn try_from(json: JsonValue) -> Result<Value, Error> {
        let v = match json {
            JsonValue::Null => Value::Empty,
            JsonValue::Bool(b) => Value::Bool(b),
            JsonValue::Number(n) => {
                if let Some(v) = n.as_u64() {
                    Value::U64(v)
                } else if let Some(v) = n.as_f64() {
                    Value::F64(v)
                } else if let Some(v) = n.as_i64() {
                    Value::I64(v)
                } else {
                    return Err(Error::IncompatibleJson(JsonValue::Number(n).into()));
                }
            }
            JsonValue::String(s) => Value::String(s.clone()),
            JsonValue::Array(_) => return Err(Error::IncompatibleJson(json.into())),
            JsonValue::Object(_) => return Err(Error::IncompatibleJson(json.into())),
        };

        Ok(v)
    }
}

impl TryFrom<Value> for JsonValue {
    type Error = Error;

    fn try_from(value: Value) -> Result<JsonValue, Error> {
        let json = match value {
            Value::I64(val) => JsonValue::Number(serde_json::Number::from(val)),
            Value::Keypair(s) => JsonValue::String(s.0),
            Value::String(s) => JsonValue::String(s),
            Value::NodeId(val) => JsonValue::String(val.to_string()),
            Value::DeletedNode(val) => JsonValue::String(val.to_string()),
            Value::Pubkey(p) => JsonValue::String(p.to_string()),
            Value::Success(s) => JsonValue::String(s.to_string()),
            Value::Balance(val) => JsonValue::Number(serde_json::Number::from(val)),
            Value::U8(val) => JsonValue::Number(serde_json::Number::from(val)),
            Value::U16(val) => JsonValue::Number(serde_json::Number::from(val)),
            Value::U64(val) => JsonValue::Number(serde_json::Number::from(val)),
            Value::F32(val) => JsonValue::Number(
                serde_json::Number::from_f64(val as f64)
                    .ok_or_else(|| Error::IncompatibleValue(Value::F32(val)))?,
            ),
            Value::F64(val) => JsonValue::Number(
                serde_json::Number::from_f64(val)
                    .ok_or_else(|| Error::IncompatibleValue(Value::F64(val)))?,
            ),
            Value::Bool(b) => JsonValue::Bool(b),
            Value::StringOpt(val) => match val {
                Some(val) => JsonValue::String(val),
                None => JsonValue::Null,
            },
            Value::Empty => JsonValue::Null,
            Value::NodeIdOpt(val) => match val {
                Some(val) => JsonValue::String(val.to_string()),
                None => JsonValue::Null,
            },
            Value::NftCreators(val) => serde_json::to_value(val).unwrap(),
            Value::MetadataAccountData(val) => serde_json::to_value(val).unwrap(),
            Value::Uses(val) => serde_json::to_value(val).unwrap(),
            Value::NftMetadata(val) => serde_json::to_value(val).unwrap(),
            Value::Operator(op) => JsonValue::String(format!("{:?}", op)),
            Value::Json(json) => json.into(),
            Value::Cancel => JsonValue::Null,
        };

        Ok(json)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonValueWrapper(JsonValue);

impl fmt::Display for JsonValueWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(&self.0).unwrap())
    }
}

impl From<JsonValue> for JsonValueWrapper {
    fn from(v: JsonValue) -> JsonValueWrapper {
        JsonValueWrapper(v)
    }
}

impl From<JsonValueWrapper> for JsonValue {
    fn from(w: JsonValueWrapper) -> JsonValue {
        w.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftMetadata {
    pub name: String,
    pub symbol: String,
    pub description: String,
    pub seller_fee_basis_points: u16,
    pub image: String,
    pub animation_url: Option<String>,
    pub external_url: Option<String>,
    pub attributes: Vec<NftMetadataAttribute>,
    pub collection: Option<NftMetadataCollection>,
    pub properties: Option<NftMetadataProperties>,
}

impl fmt::Display for NftMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(&self).unwrap())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftMetadataAttribute {
    pub trait_type: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftMetadataCollection {
    pub name: String,
    pub family: String,
    pub key: Pubkey,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftMetadataProperties {
    pub files: Option<Vec<NftMetadataFile>>,
    pub category: Option<String>,
    pub creators: Option<Vec<NftCreator>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftMetadataFile {
    pub uri: String,
    #[serde(rename = "type")]
    pub kind: String,
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
            Value::I64(_) => ValueKind::Integer,
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
            Value::Uses(_) => ValueKind::Uses,
            Value::NftMetadata(_) => ValueKind::NftMetadata,
            Value::Operator(_) => ValueKind::Operator,
            Value::Json(_) => ValueKind::Json,
            Value::Cancel => ValueKind::Cancel,
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
    Uses,
    NftMetadata,
    Operator,
    Json,
    Cancel,
}

#[derive(Debug, Deserialize, Serialize, Clone, derive_more::Display)]
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
    run_id: Uuid,
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
