use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction, message::Message, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use sunshine_core::{msg::GraphId, store::Datastore};

use crate::{error::Error, Msg};

use self::{
    account::{command_create_account, CreateAccount},
    generate_keypair::GenerateKeypair,
    token::{command_create_token, command_mint, CreateToken, MintToken},
    transfer::{command_transfer, Transfer},
};

pub mod account;
pub mod delete_keypair;
pub mod generate_keypair;
pub mod token;
pub mod transfer;

pub const KEYPAIR_NAME_MARKER: &str = "KEYPAIR_NAME_MARKER";

struct Ctx {
    client: RpcClient,
    keyring: DashMap<String, Arc<Keypair>>,
    pub_keys: DashMap<String, Pubkey>,
    key_graph: GraphId,
    db: Arc<dyn Datastore>,
}

struct Config {
    url: String,
    keyring: HashMap<String, (String, String)>,
    pub_keys: HashMap<String, String>,
    db: Arc<dyn Datastore>,
    key_graph: GraphId,
}

impl Ctx {
    fn new(cfg: Config) -> Result<Ctx, Error> {
        let keyring = cfg
            .keyring
            .into_iter()
            .map(|(name, gen_keypair)| {
                let keypair = generate_keypair::generate_keypair(&gen_keypair.0, &gen_keypair.1)?;

                println!("pubkey: {}", keypair.pubkey());

                Ok((name, Arc::new(keypair)))
            })
            .collect::<Result<DashMap<_, _>, Error>>()?;

        let pub_keys = cfg
            .pub_keys
            .into_iter()
            .map(|(name, pubkey)| {
                Ok((name, Pubkey::from_str(&pubkey).map_err(Error::ParsePubKey)?))
            })
            .chain(
                keyring
                    .iter()
                    .map(|kp| Ok((kp.key().clone(), kp.value().pubkey()))),
            )
            .collect::<Result<DashMap<_, _>, Error>>()?;

        Ok(Ctx {
            client: RpcClient::new(cfg.url),
            keyring,
            pub_keys,
            key_graph: cfg.key_graph,
            db: cfg.db,
        })
    }

    fn get_keypair(&self, name: &str) -> Result<Arc<Keypair>, Error> {
        self.keyring
            .get(name)
            .map(|r| r.value().clone())
            .ok_or(Error::KeypairDoesntExist)
    }

    fn get_pubkey(&self, name: &str) -> Result<Pubkey, Error> {
        self.pub_keys
            .get(name)
            .map(|pk| *pk)
            .ok_or(Error::PubkeyDoesntExist)
    }
}

pub struct Command {
    ctx: Arc<Mutex<Ctx>>,
    kind: Kind,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CommandResponse {
    Success,
    Balance(u64),
}
// type CommandResult = Result<(u64, Vec<Instruction>), Error>;
// type Error = Box<dyn std::error::Error>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Kind {
    GenerateKeypair(generate_keypair::GenerateKeypair),
    DeleteKeypair(delete_keypair::DeleteKeypair),
    AddPubkey(Option<keypair::AddPubConfig>),
    DeletePubkey(String),
    CreateAccount(CreateAccount),
    GetBalance(String),
    CreateToken(CreateToken),
    MintToken(MintToken),
    RequestAirdrop(String, u64),
    Transfer(Transfer),
}

impl Command {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, Msg>,
    ) -> Result<HashMap<String, Msg>, Error> {
        match self.kind {
            Kind::GenerateKeypair(k) => k.run(inputs).await,
            Kind::DeleteKeypair(k) => k.run(inputs).await,
            Kind::AddPubkey(name, pubkey) => {
                if exec_ctx.pub_keys.contains_key(name) {
                    return Err(Box::new(CustomError::PubkeyAlreadyExists));
                }

                let pubkey = Pubkey::from_str(pubkey)?;

                exec_ctx.pub_keys.insert(name.clone(), pubkey);

                Ok(CommandResponse::Success)
            }
            _ => (), /*
                     Kind::AddPubkey(name, pubkey) => {
                         if exec_ctx.pub_keys.contains_key(name) {
                             return Err(Box::new(CustomError::PubkeyAlreadyExists));
                         }

                         let pubkey = Pubkey::from_str(pubkey)?;

                         exec_ctx.pub_keys.insert(name.clone(), pubkey);

                         Ok(CommandResponse::Success)
                     }
                     Kind::DeletePubkey(k) => k.run(),
                     Kind::CreateAccount(create_account) => {
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
                     Kind::GetBalance(name) => {
                         let pubkey = exec_ctx.get_pubkey(&name)?;

                         let balance = exec_ctx.client.get_balance(&pubkey)?;

                         Ok(CommandResponse::Balance(balance))
                     }
                     Kind::CreateToken(create_token) => {
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
                     Kind::RequestAirdrop(name, amount) => {
                         let pubkey = exec_ctx.get_pubkey(&name)?;

                         exec_ctx.client.request_airdrop(&pubkey, *amount)?;

                         Ok(CommandResponse::Success)
                     }
                     Kind::MintToken(mint_token) => {
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
                     }*/
        }
    }
}

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
