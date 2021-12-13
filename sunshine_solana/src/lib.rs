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

use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::sleep;

use errors::CustomError;

use crate::commands::token::CreateToken;

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

    async fn read_flow(&self, flow_id: FlowId) -> Result<Flow, Error> {
        let graph = self
            .db
            .execute(Action::Query(QueryKind::ReadGraph(flow_id)))
            .await
            .map_err(Box::new)?
            .into_graph()
            .unwrap();

        let nodes = futures::stream::iter(graph.nodes.iter()));

        let nodes: Result<HashMap<_, _>, Error> = nodes
            .then(|node| {
                let tx = tx.clone();
                async move {
                    let cmd: Command =
                        serde_json::from_value(node.properties.get(COMMAND_MARKER).unwrap().clone())
                            .unwrap();

                    Ok((
                        node.node_id,
                        Node {
                            inputs: Vec::new(),
                            outputs: Vec::new(),
                            cmd,
                        },
                    ))
                }
            })
            .try_collect()
            .await;

        let nodes = nodes?;

        let mut start_nodes = Vec::new();

        for node in graph.nodes.iter() {
            for edge in node.outbound_edges.iter() {
                let (tx, rx) = mpsc::unbounded();
                nodes.get_mut(edge.from).unwrap().outputs.push(tx);
                nodes.get_mut(edge.to).unwrap().inputs.push(rx);
            }
            if node.properties.contains_key(START_NODE_MARKER) {
                let (tx, rx) = mspc::unbounded();
                nodes.get_mut(node.node_id).unwrap().inputs.push(rx);
                start_nodes.push(tx);
            }
        }

        Ok(Flow {
            start_nodes,
            nodes,
        })
    }

    async fn deploy_flow(&self, period: Duration, flow_id: FlowId) -> Result<(), Error> {
        //let flow = self.read_flow(flow_id).await?;

        println!("{:#?}", flow);

        let mut interval = tokio::time::interval(period);

        let flow = Arc::new(flow);
        let exec_ctx = self.exec_ctx.clone();

        let interval_fut = async move {
            while let _ = interval.tick().await {
                let flow = flow.clone();
                let exec_ctx = exec_ctx.clone();

                println!("tick");

                let other = self.clone();

                let Flow {
                    nodes, start_nodes
                } = match other.read_flow(flow_id).await {
                    Ok(flow) => flow,
                    Err(e) => {
                        eprintln!("failed to read flow: {}", e);
                        return;
                    }
                };

                for node in nodes {
                    tokio::spawn(async move {
                        // join inputs 
                        // execute cmd
                        // send outputs
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
    nodes: HashMap<NodeId, Node>,
}

struct Node {
    inputs: Vec<Receiver<Msg>>,
    outputs: Vec<Sender<Msg>>,
    cmd: Command,
}

// A
// 0        //*/C          //E
// B
// D

// edge
// nodesA    input1,2,3 processing output 1,2
// nodeB     input1                output1

// node addition  input 1,2        output 1

// https://rust-lang.github.io/async-book/07_workarounds/04_recursion.html

impl Flow {
    pub async fn run(self: Arc<Self>, exec_ctx: Arc<ExecutionContext>) {
        for cmd_id in self.start_commands.iter() {
            self.clone().run_entry(*cmd_id, exec_ctx.clone()).await;
        }
    }

    fn run_entry(
        self: Arc<Self>,
        id: CommandId,
        exec_ctx: Arc<ExecutionContext>,
        cmd_res: CommandResult,
    ) -> BoxFuture<'static, ()> {
        async move {
            let entry = self.commands.get(&id).unwrap();

            println!("RUNNING COMMAND");

            let cmd_res = match self
                .clone()
                .run_command(&entry.command, exec_ctx.clone())
                .await
            {
                Ok(res) => res,
                Err(e) => {
                    eprintln!("error while running entry: {}", e);
                    return;
                }
            };

            for next in entry.next.iter() {
                match next.cond {
                    Some(true) | None => {
                        let exec_ctx = exec_ctx.clone();
                        let other = self.clone();
                        let id: CommandId = next.id;
                        tokio::spawn(async move { other.run_entry(id, exec_ctx).await });
                    }
                    Some(false) => (),
                }
            }
        }
        .boxed()
    }

    async fn run_command(
        &self,
        cmd: &Command,
        exec_ctx: Arc<ExecutionContext>,
    ) -> Result<CommandResponse, Error> {
        println!("{:#?}", cmd);

        match cmd {
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
                }

                if exec_ctx.pub_keys.remove(name).is_none() {
                    return Err(Box::new(CustomError::PubkeyDoesntExist));
                }

                Ok(CommandResponse::Success)
            }
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
        }
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

const START_ENTRY_MARKER: &str = "START_ENTRY_MARKER";
const COMMAND_MARKER: &str = "COMMAND_MARKER";

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

    props.insert(START_COMMAND_MARKER.into(), JsonValue::Bool(true));
    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&Command::Print("hello1".into())).unwrap(),
    );

    let node1 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 2
    let mut props = serde_json::Map::new();

    props.insert(START_COMMAND_MARKER.into(), JsonValue::Bool(true));
    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&Command::Print("hello2".into())).unwrap(),
    );

    let node2 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 3
    let mut props = serde_json::Map::new();

    props.insert(START_COMMAND_MARKER.into(), JsonValue::Bool(true));
    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&Command::Print("hello3".into())).unwrap(),
    );

    let node3 = store
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
                to: node2,
                properties: Default::default(),
            }),
        ))
        .await
        .unwrap();

    store
        .execute(Action::Mutate(
            graph_id,
            MutateKind::CreateEdge(CreateEdge {
                from: node1,
                to: node3,
                properties: Default::default(),
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
