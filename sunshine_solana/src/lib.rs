use solana_sdk::signature::Signature;
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error as ThisError;

use commands::{simple, Command};
use dashmap::DashMap;
use futures::future::{BoxFuture, FutureExt};
use futures::stream::{Stream, StreamExt, TryStreamExt};
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use solana_client::rpc_client::RpcClient;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;
use sunshine_core::msg::CreateEdge;
use sunshine_core::msg::MutateKind;
use sunshine_core::msg::{Action, GraphId, NodeId, QueryKind};
use sunshine_core::store::Datastore;
use sunshine_indra::store::DbConfig;
use sunshine_indra::store::DB;

use tokio::sync::mpsc::{self, UnboundedReceiver as Receiver, UnboundedSender as Sender};
use tokio::sync::oneshot;
use tokio::time::sleep;

use serde_json::Value;
use tokio::task::spawn_blocking;
use tokio::time::Duration;
use uuid::Uuid;

mod commands;
mod error;
use error::Error;

type FlowId = GraphId;
type CommandId = NodeId;

type CommandResult = Result<(u64, Vec<Instruction>), Error>;

const START_NODE_MARKER: &str = "START_NODE_MARKER";
const COMMAND_MARKER: &str = "COMMAND_MARKER";
const INPUT_ARG_NAME_MARKER: &str = "INPUT_ARG_NAME_MARKER";
const OUTPUT_ARG_NAME_MARKER: &str = "OUTPUT_ARG_NAME_MARKER";

#[derive(Debug)]
struct Config {
    url: String,
}

struct FlowContext {
    deployed: DashMap<FlowId, oneshot::Sender<()>>,
    db: Arc<dyn Datastore>,
}

impl FlowContext {
    fn new(cfg: Config, db: Arc<dyn Datastore>) -> FlowContext {
        FlowContext {
            deployed: DashMap::new(),
            db,
        }
    }

    fn undeploy_flow(&self, flow_id: FlowId) -> Result<(), Error> {
        let (_, stop_signal) = self
            .deployed
            .remove(&flow_id)
            .ok_or(Error::FlowDoesntExist)?;
        stop_signal.send(()).unwrap();
        Ok(())
    }

    // TODO not taking mutable reference

    async fn read_flow(db: Arc<dyn Datastore>, flow_id: FlowId) -> Result<Flow, Error> {
        let graph = db
            .execute(Action::Query(QueryKind::ReadGraph(flow_id)))
            .await?
            .into_graph()
            .unwrap();

        let nodes = futures::stream::iter(graph.nodes.iter());

        let mut nodes = {
            let nodes: Result<HashMap<_, _>, Error> = nodes
                .then(|node| async move {
                    let cmd: Command = serde_json::from_value(
                        node.properties.get(COMMAND_MARKER).unwrap().clone(),
                    )
                    .unwrap();

                    Ok((
                        node.node_id,
                        FlowNode {
                            inputs: HashMap::new(),
                            outputs: HashMap::new(),
                            cmd,
                        },
                    ))
                })
                .try_collect()
                .await;

            nodes?
        };

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

    async fn deploy_flow(&self, period: Duration, flow_id: FlowId) -> Result<(), Error> {
        let mut interval = tokio::time::interval(period);

        let db = self.db.clone();

        let interval_fut = async move {
            while let _ = interval.tick().await {
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
                    node.send(OutputType::default()).unwrap();
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

type EntryId = NodeId;

#[derive(Debug)]
enum OutputType {
    Integer(i64),
    Keypair(Keypair),
    String(String),
    NodeId(Uuid),
    DeletedNode(Uuid),
    Pubkey(Pubkey),
    Success(Signature),
    Balance(u64),
    U8(u8),
    U64(u64),
    Float(f64),
}

struct Flow {
    start_nodes: Vec<Sender<OutputType>>,
    nodes: HashMap<NodeId, FlowNode>,
}

struct FlowNode {
    inputs: HashMap<String, Receiver<OutputType>>,
    outputs: HashMap<String, Vec<Sender<OutputType>>>,
    cmd: Command,
}

async fn run_command(
    cmd: &Command,
    inputs: HashMap<String, OutputType>,
) -> Result<HashMap<String, OutputType>, Error> {
    println!("{:#?}", cmd);

    println!("{:#?}", inputs);

    match cmd {
        Command::Simple(simple) => simple.run(inputs).await,

        _ => unreachable!(),
    }
}

// flow api

// gui app
