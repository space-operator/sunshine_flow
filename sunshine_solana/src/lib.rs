use std::str::FromStr;
use std::sync::Arc;

use commands::account::command_create_account;
use commands::keypair::generate_keypair;
use commands::keypair::GenerateKeypair;
use commands::token::command_create_token;
use commands::token::command_mint;
use commands::transfer::command_transfer;
use commands::Command;
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

use errors::CustomError;

use crate::commands::token::CreateToken;

use serde_json::Value;
use tokio::task::spawn_blocking;
use tokio::time::Duration;

mod commands;
mod errors;

type FlowId = GraphId;
type CommandId = NodeId;

type CommandResult = Result<(u64, Vec<Instruction>), Error>;
type Error = Box<dyn std::error::Error>;

#[derive(Debug, Serialize, Deserialize)]
pub enum CommandResponse {
    Success,
    Balance(u64),
}

#[derive(Debug)]
struct Config {
    url: String,
    keyring: HashMap<String, GenerateKeypair>,
    pub_keys: HashMap<String, String>,
}

struct FlowContext {
    exec_ctx: Arc<ExecutionContext>,
    deployed: DashMap<FlowId, oneshot::Sender<()>>,
    db: Arc<dyn Datastore>,
}

impl FlowContext {
    fn new(cfg: Config, db: Arc<dyn Datastore>) -> Result<FlowContext, Error> {
        let keyring = cfg
            .keyring
            .into_iter()
            .map(|(name, gen_keypair)| {
                let keypair = generate_keypair(&gen_keypair.passphrase, &gen_keypair.seed_phrase)?;

                println!("pubkey: {}", keypair.pubkey());

                Ok((name, Arc::new(keypair)))
            })
            .collect::<Result<DashMap<_, _>, Error>>()?;

        let pub_keys = cfg
            .pub_keys
            .into_iter()
            .map(|(name, pubkey)| Ok((name, Pubkey::from_str(&pubkey)?)))
            .chain(
                keyring
                    .iter()
                    .map(|kp| Ok((kp.key().clone(), kp.value().pubkey()))),
            )
            .collect::<Result<DashMap<_, _>, Error>>()?;

        Ok(FlowContext {
            exec_ctx: Arc::new(ExecutionContext {
                client: RpcClient::new(cfg.url),
                keyring,
                pub_keys,
            }),
            deployed: DashMap::new(),
            db,
        })
    }

    fn undeploy_flow(&self, flow_id: FlowId) -> Result<(), Error> {
        let (_, stop_signal) = self
            .deployed
            .remove(&flow_id)
            .ok_or(Box::new(CustomError::FlowDoesntExist))?;
        stop_signal.send(()).unwrap();
        Ok(())
    }

    // TODO not taking mutable reference

    async fn read_flow(db: Arc<dyn Datastore>, flow_id: FlowId) -> Result<Flow, Error> {
        let graph = db
            .execute(Action::Query(QueryKind::ReadGraph(flow_id)))
            .await
            .map_err(Box::new)?
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
                nodes
                    .get_mut(&edge.from)
                    .unwrap()
                    .outputs
                    .insert(output_arg_name.to_owned(), tx);
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

        let exec_ctx = self.exec_ctx.clone();
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
                    let exec_ctx = exec_ctx.clone();
                    tokio::spawn(async move {
                        let mut inputs = HashMap::new();
                        for (name, mut rx) in node.inputs {
                            inputs.insert(name, rx.recv().await.unwrap());
                        }

                        let mut outputs = run_command(exec_ctx, &node.cmd, inputs).await.unwrap();
                        assert!(outputs.len() >= node.outputs.len());

                        for (name, tx) in node.outputs.into_iter() {
                            tx.send(outputs.remove(&name).unwrap()).unwrap();
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

struct ExecutionContext {
    client: RpcClient,
    keyring: DashMap<String, Arc<Keypair>>,
    pub_keys: DashMap<String, Pubkey>,
}

impl ExecutionContext {
    fn get_keypair(&self, name: &str) -> Result<Arc<Keypair>, Error> {
        self.keyring
            .get(name)
            .map(|r| r.value().clone())
            .ok_or(Box::new(CustomError::KeypairDoesntExist))
    }

    fn get_pubkey(&self, name: &str) -> Result<Pubkey, Error> {
        self.pub_keys
            .get(name)
            .map(|pk| *pk)
            .ok_or(Box::new(CustomError::PubkeyDoesntExist))
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
    outputs: HashMap<String, Sender<Msg>>,
    cmd: Command,
}

async fn run_command(
    exec_ctx: Arc<ExecutionContext>,
    cmd: &Command,
    mut inputs: HashMap<String, Msg>,
) -> Result<HashMap<String, Msg>, Error> {
    println!("{:#?}", cmd);

    println!("{:#?}", inputs);

    match cmd {
        Command::Add => {
            let a = inputs.remove("a").unwrap();
            let b = inputs.remove("b").unwrap();

            Ok(hashmap! {
                "res".to_owned() => a + b,
            })
        }
        Command::Print => {
            let p = inputs.remove("p").unwrap();

            println!("{:#?}", p);

            Ok(hashmap! {
                "res".to_owned() => p,
            })
        }
        Command::Const(msg) => Ok(hashmap! {
            "res".to_owned() => *msg,
        }),
        _ => unreachable!(),
        /*
        Command::GenerateKeypair(name, gen_keypair) => {
            if exec_ctx.keyring.contains_key(name) {
                return Err(Box::new(CustomError::KeypairAlreadyExistsInKeyring));
            }

            if exec_ctx.pub_keys.contains_key(name) {
                return Err(Box::new(CustomError::PubkeyAlreadyExists));
            }

            let keypair = generate_keypair(&gen_keypair.passphrase, &gen_keypair.seed_phrase)?;
            // let keypair = Arc::new(keypair);

            exec_ctx.pub_keys.insert(name.clone(), keypair.pubkey());
            exec_ctx.keyring.insert(name.clone(), Arc::new(keypair));

            Ok(CommandResponse::Success)
        }
        Command::DeleteKeypair(name) => {
            if exec_ctx.keyring.remove(name).is_none() {
                return Err(Box::new(CustomError::KeypairDoesntExist));
            }maplit
        Command::AddPubkey(name, pubkey) => {
            if exec_ctx.pub_keys.contains_key(name) {
                return Err(Box::new(CustomError::PubkeyAlreadyExists));
            }

            let pubkey = Pubkey::from_str(pubkey)?;

            exec_ctx.pub_keys.insert(name.clone(), pubkey);

            Ok(CommandResponse::Success)
        }
        Command::DeletePubkey(name) => {
            if exec_ctx.pub_keys.remove(name).is_none() {
                return Err(Box::new(CustomError::PubkeyDoesntExist));
            }

            Ok(CommandResponse::Success)
        }
        Command::CreateAccount(create_account) => {
            let owner = exec_ctx.get_pubkey(&create_account.owner)?;
            let fee_payer = exec_ctx.get_keypair(&create_account.fee_payer)?;
            let token = exec_ctx.get_pubkey(&create_account.token)?;
            let account = match create_account.account {
                Some(ref account) => Some(exec_ctx.get_keypair(account)?),
                None => None,
            };

            let (minimum_balance_for_rent_exemption, instructions) = command_create_account(
                &exec_ctx.client,
                fee_payer.pubkey(),
                token,
                owner,
                account.as_ref().map(|a| a.pubkey()),
            )
            .unwrap();

            let mut signers: Vec<Arc<dyn Signer>> = vec![fee_payer.clone()];

            if let Some(account) = account {
                signers.push(account.clone());
            };

            execute_instructions(
                &signers,
                &exec_ctx.client,
                &fee_payer.pubkey(),
                &instructions,
                minimum_balance_for_rent_exemption,
            )?;

            Ok(CommandResponse::Success)
        }
        Command::GetBalance(name) => {
            let pubkey = exec_ctx.get_pubkey(&name)?;

            let balance = exec_ctx.client.get_balance(&pubkey)?;

            Ok(CommandResponse::Balance(balance))
        }
        Command::CreateToken(create_token) => {
            let fee_payer = exec_ctx.get_keypair(&create_token.fee_payer)?;
            let authority = exec_ctx.get_keypair(&create_token.authority)?;
            let token = exec_ctx.get_keypair(&create_token.token)?;

            let (minimum_balance_for_rent_exemption, instructions) = command_create_token(
                &exec_ctx.client,
                &fee_payer.pubkey(),
                create_token.decimals,
                &token.pubkey(),
                authority.pubkey(),
                &create_token.memo,
            )?;

            let signers: Vec<Arc<dyn Signer>> =
                vec![authority.clone(), fee_payer.clone(), token.clone()];

            execute_instructions(
                &signers,
                &exec_ctx.client,
                &fee_payer.pubkey(),
                &instructions,
                minimum_balance_for_rent_exemption,
            )?;

            Ok(CommandResponse::Success)
        }
        Command::RequestAirdrop(name, amount) => {
            let pubkey = exec_ctx.get_pubkey(&name)?;

            exec_ctx.client.request_airdrop(&pubkey, *amount)?;

            Ok(CommandResponse::Success)
        }
        Command::MintToken(mint_token) => {
            let token = exec_ctx.get_keypair(&mint_token.token)?;
            let mint_authority = exec_ctx.get_keypair(&mint_token.mint_authority)?;
            let recipient = exec_ctx.get_pubkey(&mint_token.recipient)?;
            let fee_payer = exec_ctx.get_keypair(&mint_token.fee_payer)?;

            let (minimum_balance_for_rent_exemption, instructions) = command_mint(
                &exec_ctx.client,
                token.pubkey(),
                mint_token.amount,
                recipient,
                mint_authority.pubkey(),
            )?;

            let signers: Vec<Arc<dyn Signer>> =
                vec![mint_authority.clone(), token.clone(), fee_payer.clone()];

            execute_instructions(
                &signers,
                &exec_ctx.client,
                &fee_payer.pubkey(),
                &instructions,
                minimum_balance_for_rent_exemption,
            )?;

            Ok(CommandResponse::Success)
        }
        Command::Transfer(transfer) => {
            let token = exec_ctx.get_pubkey(&transfer.token)?;
            let recipient = exec_ctx.get_pubkey(&transfer.recipient)?;
            let fee_payer = exec_ctx.get_keypair(&transfer.fee_payer)?;
            let sender = match transfer.sender {
                Some(ref sender) => Some(exec_ctx.get_keypair(sender)?),
                None => None,
            };
            let sender_owner = exec_ctx.get_keypair(&transfer.sender_owner)?;

            let (minimum_balance_for_rent_exemption, instructions) = command_transfer(
                &exec_ctx.client,
                &fee_payer.pubkey(),
                token,
                transfer.amount,
                recipient,
                sender.as_ref().map(|s| s.pubkey()),
                sender_owner.pubkey(),
                transfer.allow_unfunded_recipient,
                transfer.fund_recipient,
                transfer.memo.clone(),
            )?;

            let mut signers: Vec<Arc<dyn Signer>> =
                vec![fee_payer.clone(), sender_owner.clone()];

            if let Some(sender) = sender {
                signers.push(sender);
            }

            execute_instructions(
                &signers,
                &exec_ctx.client,
                &fee_payer.pubkey(),
                &instructions,
                minimum_balance_for_rent_exemption,
            )?;

            Ok(CommandResponse::Success)
        }
        Command::Print(s) => {
            println!("{}", s);
            Ok(CommandResponse::Success)
        }
        */
    }
}

fn execute_instructions(
    signers: &Vec<Arc<dyn Signer>>,
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

// flow api

// gui app

const START_NODE_MARKER: &str = "START_NODE_MARKER";
const COMMAND_MARKER: &str = "COMMAND_MARKER";
const INPUT_ARG_NAME_MARKER: &str = "INPUT_ARG_NAME_MARKER";
const OUTPUT_ARG_NAME_MARKER: &str = "OUTPUT_ARG_NAME_MARKER";

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
        serde_json::to_value(&Command::Const(3)).unwrap(),
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
        serde_json::to_value(&Command::Const(2)).unwrap(),
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
        serde_json::to_value(&Command::Add).unwrap(),
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
        serde_json::to_value(&Command::Print).unwrap(),
    );

    let node4 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

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

    flow_ctx
        .deploy_flow(Duration::from_secs(5), graph_id)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(11)).await;
}
