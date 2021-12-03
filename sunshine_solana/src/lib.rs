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
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;
use tokio::sync::oneshot;
use tokio::time::sleep;

use errors::CustomError;

use crate::commands::token::CreateToken;

use tokio::task::spawn_blocking;
use tokio::time::Duration;

mod commands;
mod errors;

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
    deployed: HashMap<String, oneshot::Sender<()>>,
    stored: HashMap<String, Flow>,
}

impl FlowContext {
    fn new(cfg: Config) -> Result<FlowContext, Error> {
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
            deployed: HashMap::new(),
            stored: HashMap::new(),
        })
    }

    fn store_flow(&mut self, name: &str, flow: Flow) {
        self.stored.insert(name.to_owned(), flow);
    }

    fn undeploy_flow(&mut self, name: &str) -> Result<(), Error> {
        let stop_signal = self
            .deployed
            .remove(name)
            .ok_or(Box::new(CustomError::FlowDoesntExist))?;
        stop_signal.send(()).unwrap();
        Ok(())
    }

    fn deploy_flow(&mut self, period: Duration, name: &str) -> Result<(), Error> {
        let flow = self
            .stored
            .get(name)
            .ok_or(Box::new(CustomError::FlowDoesntExist))?
            .clone();

        let mut interval = tokio::time::interval(period);

        let flow = Arc::new(flow);
        let exec_ctx = self.exec_ctx.clone();

        let interval_fut = async move {
            while let _ = interval.tick().await {
                let flow = flow.clone();
                let exec_ctx = exec_ctx.clone();

                println!("tick");

                tokio::spawn(async move {
                    let res = tokio::task::spawn_blocking(move || {
                        flow.run(exec_ctx.clone()).map_err(|e| e.to_string())
                    })
                    .await
                    .unwrap();

                    if let Err(e) = res {
                        eprintln!("error while running flow: {}", e);
                    }
                });
            }
        };

        let (send_stop_signal, stop_signal) = oneshot::channel();

        tokio::spawn(async {
            tokio::select! {
                _ = interval_fut => (),
                _ = stop_signal => (),
            }
        });

        self.deployed.insert(name.to_owned(), send_stop_signal);

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

#[derive(Clone)]
struct Flow {
    commands: Vec<Command>,
}

impl Flow {
    pub fn run(&self, exec_ctx: Arc<ExecutionContext>) -> Result<CommandResponse, Error> {
        for cmd in self.commands.iter() {
            self.run_command(cmd, exec_ctx.clone())?;
        }

        Ok(CommandResponse::Success)
    }

    fn run_command(
        &self,
        cmd: &Command,
        exec_ctx: Arc<ExecutionContext>,
    ) -> Result<CommandResponse, Error> {
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

#[tokio::test(flavor = "multi_thread")]
async fn test_flow_ctx() {
    let mut keyring = HashMap::new();

    keyring.insert(
        "me".to_owned(),
        GenerateKeypair {
            passphrase: "pass".into(),
            seed_phrase: "beach soldier piano click essay sock stable cover angle wear aunt advice"
                .into(),
        },
    );

    let mut ctx = FlowContext::new(Config {
        url: "https://api.devnet.solana.com".to_owned(),
        keyring,
        pub_keys: HashMap::new(),
    })
    .unwrap();

    let flow = Flow {
        commands: vec![Command::RequestAirdrop("me".to_owned(), 1_000_000_000)],
    };

    ctx.store_flow("flow1", flow);

    ctx.deploy_flow(Duration::from_secs(5), "flow1").unwrap();

    ctx.undeploy_flow("flow1").unwrap();

    // https://explorer.solana.com/address/9B5XszUGdMaxCZ7uSQhPzdks5ZQSmWxrmzCSvtJ6Ns6g?cluster=devnet
    // https://explorer.solana.com/address/7W3KHiYzPZjy2Be4NyZQi1PDQE152MXrBbivYKGLsmrS?cluster=devnet
    // https://explorer.solana.com/address/7zq7kpQ5u9TYQVq6nWBbnWui9bACVGsd9dCrFYkAGH6M?cluster=devnet
    //https://github.com/solana-labs/token-list
    //https://github.com/solana-labs/solana/blob/b8ac6c1889d93e10967ddac850f9dd8c5b1c5c95/explorer/src/pages/AccountDetailsPage.tsx
    // 1. wallet
    //      add accounts
    // 2. create token account

    //let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
    //panic!("{}", mnemonic.phrase());
    /*
    let seed_phrase = "beach soldier piano click essay sock stable cover angle wear aunt advice";
    let keypair = generate_keypair("", seed_phrase).unwrap();
    let client = RpcClient::new("https://api.devnet.solana.com".to_owned());
    let user = keypair.pubkey();
    let seed_phrase = "guard gun term bless spare iron miss flee solid forum bring will";
    let token_keypair = generate_keypair("", seed_phrase).unwrap();
    let token = token_keypair.pubkey();
    println!("Creating token {token}");
    let seed_phrase = "risk foster path suit lecture fit ancient allow major reward open favorite";
    let custom_token_account_keypair = generate_keypair("", seed_phrase).unwrap();
    let custom_token_account = custom_token_account_keypair.pubkey();
    println!("custom token account1: {custom_token_account}");
    let seed_phrase =
        "property space future road athlete various frame doll evolve stuff aim hidden";
    let custom_token_account_keypair2 = generate_keypair("", seed_phrase).unwrap();
    let custom_token_account2 = custom_token_account_keypair2.pubkey();
    println!("custom token account2: {custom_token_account2}");
    println!("creating account: {custom_token_account2}");
    let (minimum_balance_for_rent_exemption, instructions) =
        command_create_account(&client, user, token, user, Some(custom_token_account2)).unwrap();
    let signers: Vec<&dyn Signer> = vec![&keypair, &custom_token_account_keypair2];
    execute_instructions(
        &signers,
        &client,
        &user,
        &instructions,
        minimum_balance_for_rent_exemption,
    );
    println!("Minting token {token}");
    let (minimum_balance_for_rent_exemption, instructions) =
        command_mint(&client, token, 120.0, custom_token_account, user).unwrap();
    let signers: Vec<&dyn Signer> = vec![&keypair, &token_keypair];
    execute_instructions(
        &signers,
        &client,
        &user,
        &instructions,
        minimum_balance_for_rent_exemption,
    );
    println!("sending money from {custom_token_account} to {custom_token_account2}");
    let (minimum_balance_for_rent_exemption, instructions) = command_transfer(
        &client,
        &custom_token_account,
        token,
        24.0,
        custom_token_account2,
        Some(custom_token_account),
        user,
        true,
        true,
        Some("SENDING MONEY TO SECOND ACCOUNT".to_owned()),
    )
    .unwrap();
    let signers: Vec<&dyn Signer> = vec![&keypair, &custom_token_account_keypair];
    execute_instructions(
        &signers,
        &client,
        &user,
        &instructions,
        minimum_balance_for_rent_exemption,
    ).unwrap();
    */
}
