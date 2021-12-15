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

mod commands;
mod error;

use error::Error;

type FlowId = GraphId;
type CommandId = NodeId;

type CommandResult = Result<(u64, Vec<Instruction>), Error>;

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
                    node.send(Msg::default()).unwrap();
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

type Msg = i32;

struct Flow {
    start_nodes: Vec<Sender<Msg>>,
    nodes: HashMap<NodeId, FlowNode>,
}

struct FlowNode {
    inputs: HashMap<String, Receiver<Msg>>,
    outputs: HashMap<String, Vec<Sender<Msg>>>,
    cmd: Command,
}

async fn run_command(
    cmd: &Command,
    mut inputs: HashMap<String, Msg>,
) -> Result<HashMap<String, Msg>, Error> {
    println!("{:#?}", cmd);

    println!("{:#?}", inputs);

    match cmd {
        Command::Simple(simple) => simple.run(inputs).await,
        _ => unreachable!(),
    }
}

/*
fn execute_instructions(
    signers: &[Arc<dyn Signer>],
    client: &RpcClient,
    fee_payer: &Pubkey,
    instructions: &[Instruction],
    minimum_balance_for_rent_exemption: u64,
) -> Result<(), Error> {
    /*let message = if let Some(nonce_account) = config.nonce_account.as_ref() {
        Message::new_with_nonce(
            instructions,
            fee_payer,
            nonce_account,
            config.nonce_authority.as_ref().unwrap(),
        )
    } else {
        Message::new(&instructions, fee_payer)
    };*/

    let message = Message::new(instructions, Some(fee_payer));

    let (recent_blockhash, fee_calculator) = client.get_recent_blockhash()?;

    let balance = client.get_balance(fee_payer)?;

    if balance < minimum_balance_for_rent_exemption + fee_calculator.calculate_fee(&message) {
        panic!("insufficient balance");
    }

    let mut transaction = Transaction::new_unsigned(message);

    let signers = signers
        .iter()
        .map(|s| s.as_ref())
        .collect::<Vec<&dyn Signer>>();

    transaction.try_sign(&signers, recent_blockhash)?;

    let signature = client.send_and_confirm_transaction(&transaction)?;

    Ok(())
}
*/

// flow api

// gui app

const START_NODE_MARKER: &str = "START_NODE_MARKER";
const COMMAND_MARKER: &str = "COMMAND_MARKER";
const INPUT_ARG_NAME_MARKER: &str = "INPUT_ARG_NAME_MARKER";
const OUTPUT_ARG_NAME_MARKER: &str = "OUTPUT_ARG_NAME_MARKER";

/*
#[tokio::test(flavor = "multi_thread")]
async fn test_flow_ctx() {
    let store = sunshine_indra::store::DB::new(&sunshine_indra::store::DbConfig {
        db_path: "test_indra_db_flow_ctx".to_owned(),
    })
    .unwrap();

    let store = Arc::new(store);

    let flow_ctx = FlowContext::new(
        Config {
            url: "https://api.devnet.solana.com".into(),
            keyring: HashMap::new(),
            pub_keys: HashMap::new(),
        },
        store.clone(),
    )
    .unwrap();

    let graph_id = store
        .execute(Action::CreateGraph(Default::default()))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 1
    let mut props = serde_json::Map::new();

    props.insert(START_NODE_MARKER.into(), JsonValue::Bool(true));
    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&Command::Simple(SimpleCommand::Const(3))).unwrap(),
    );

    let node1 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 2
    let mut props = serde_json::Map::new();

    props.insert(START_NODE_MARKER.into(), JsonValue::Bool(true));
    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&SimpleCommand::Const(2)).unwrap(),
    );

    let node2 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 3
    let mut props = serde_json::Map::new();

    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&SimpleCommand::Add).unwrap(),
    );

    let node3 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 4
    let mut props = serde_json::Map::new();

    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&SimpleCommand::Print).unwrap(),
    );

    let node4 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 5
    let mut props = serde_json::Map::new();

    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&SimpleCommand::Const(7)).unwrap(),
    );

    let node5 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 6
    let mut props = serde_json::Map::new();

    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&SimpleCommand::Add).unwrap(),
    );

    let node6 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 7
    let mut props = serde_json::Map::new();

    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&SimpleCommand::Print).unwrap(),
    );

    let node7 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();
    //
    store
        .execute(Action::Mutate(
            graph_id,
            MutateKind::CreateEdge(CreateEdge {
                from: node1,
                to: node3,
                properties: serde_json::json! ({
                    INPUT_ARG_NAME_MARKER: "a",
                    OUTPUT_ARG_NAME_MARKER: "res",
                })
                .as_object()
                .unwrap()
                .clone(),
            }),
        ))
        .await
        .unwrap();

    store
        .execute(Action::Mutate(
            graph_id,
            MutateKind::CreateEdge(CreateEdge {
                from: node2,
                to: node3,
                properties: serde_json::json!({
                    INPUT_ARG_NAME_MARKER: "b",
                    OUTPUT_ARG_NAME_MARKER: "res",
                })
                .as_object()
                .unwrap()
                .clone(),
            }),
        ))
        .await
        .unwrap();

    store
        .execute(Action::Mutate(
            graph_id,
            MutateKind::CreateEdge(CreateEdge {
                from: node3,
                to: node4,
                properties: serde_json::json!({
                    INPUT_ARG_NAME_MARKER: "p",
                    OUTPUT_ARG_NAME_MARKER: "res",
                })
                .as_object()
                .unwrap()
                .clone(),
            }),
        ))
        .await
        .unwrap();

    store
        .execute(Action::Mutate(
            graph_id,
            MutateKind::CreateEdge(CreateEdge {
                from: node3,
                to: node6,
                properties: serde_json::json!({
                    INPUT_ARG_NAME_MARKER: "a",
                    OUTPUT_ARG_NAME_MARKER: "res",
                })
                .as_object()
                .unwrap()
                .clone(),
            }),
        ))
        .await
        .unwrap();

    store
        .execute(Action::Mutate(
            graph_id,
            MutateKind::CreateEdge(CreateEdge {
                from: node5,
                to: node6,
                properties: serde_json::json!({
                    INPUT_ARG_NAME_MARKER: "b",
                    OUTPUT_ARG_NAME_MARKER: "res",
                })
                .as_object()
                .unwrap()
                .clone(),
            }),
        ))
        .await
        .unwrap();

    store
        .execute(Action::Mutate(
            graph_id,
            MutateKind::CreateEdge(CreateEdge {
                from: node6,
                to: node7,
                properties: serde_json::json!({
                    INPUT_ARG_NAME_MARKER: "p",
                    OUTPUT_ARG_NAME_MARKER: "res",
                })
                .as_object()
                .unwrap()
                .clone(),
            }),
        ))
        .await
        .unwrap();

    flow_ctx
        .deploy_flow(Duration::from_secs(5), graph_id)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(11)).await;
}
*/
