use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use sunshine_core::{msg::GraphId, store::Datastore};

use crate::{error::Error, OutputType};

use self::{
    token::{CreateToken, MintToken},
    transfer::Transfer,
};

pub mod instructions;

pub mod add_pubkey;
pub mod create_account;
pub mod create_token;
pub mod delete_keypair;
pub mod delete_pubkey;
pub mod generate_keypair;
pub mod get_balance;
pub mod mint_token;
pub mod request_airdrop;
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
    AddPubkey(add_pubkey::AddPubkey),
    DeletePubkey(delete_pubkey::DeletePubkey),
    CreateAccount(create_account::CreateAccount),
    GetBalance(get_balance::GetBalance),
    CreateToken(create_token::CreateToken),
    RequestAirdrop(request_airdrop::RequestAirdrop),
    MintToken(MintToken),
    Transfer(Transfer),
}

impl Command {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, OutputType>,
    ) -> Result<HashMap<String, OutputType>, Error> {
        match self.kind {
            Kind::GenerateKeypair(k) => k.run(inputs).await,
            Kind::DeleteKeypair(k) => k.run(self.ctx, inputs).await,
            Kind::AddPubkey(k) => k.run(self.ctx, inputs).await,
            Kind::DeletePubkey(k) => k.run(self.ctx, inputs).await,
            Kind::CreateAccount(k) => k.run(self.ctx, inputs).await,
            Kind::GetBalance(k) => k.run(self.ctx, inputs).await,
            Kind::CreateToken(k) => k.run(self.ctx, inputs).await,
            Kind::RequestAirdrop(k) => k.run(self.ctx, inputs).await,
            Kind::MintToken(k) => k.run(self.ctx, inputs).await,

            _ => (), /*


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
