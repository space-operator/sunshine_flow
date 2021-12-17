use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use sunshine_core::msg::{CreateEdge, Graph, GraphId, Properties};
use sunshine_core::store::Datastore;

use crate::{error::Error, ValueType};

use sunshine_core::msg::NodeId;

mod instructions;

// pub mod add_pubkey;
// pub mod create_account;
// pub mod create_token;
// pub mod delete_keypair;
// pub mod delete_pubkey;
mod generate_keypair;
mod get_balance;
// pub mod mint_token;
// pub mod request_airdrop;
// pub mod transfer;

const KEYPAIR_NAME_MARKER: &str = "KEYPAIR_NAME_MARKER";

struct Config {
    url: String,
    db: Arc<dyn Datastore>,
    wallet_graph: GraphId,
}

pub struct Ctx {
    client: RpcClient,
    db: Arc<dyn Datastore>,
    wallet_graph: GraphId,
}

const KEYPAIR_MARKER: &str = "KEYPAIR_MARKER";
const NAME_MARKER: &str = "NAME_MARKER";
const PUBKEY_MARKER: &str = "PUBKEY_MARKER";

impl Ctx {
    fn new(cfg: Config) -> Result<Ctx, Error> {
        Ok(Ctx {
            client: RpcClient::new(cfg.url),
            wallet_graph: cfg.wallet_graph,
            db: cfg.db,
        })
    }

    fn check_name(graph: &Graph, name: &str) -> Result<(), Error> {
        let has_name = graph.nodes.iter().any(|node| {
            if let Some(node_name) = node.properties.get(NAME_MARKER) {
                node_name == name
            } else {
                false
            }
        });

        if has_name {
            Err(Error::NameAlreadyInUse)
        } else {
            Ok(())
        }
    }

    async fn insert_keypair(&self, name: String, keypair: &Keypair) -> Result<NodeId, Error> {
        let graph = self.db.read_graph(self.wallet_graph).await?;

        Self::check_name(&graph, &name)?;

        let mut props = Properties::default();

        props.insert(KEYPAIR_MARKER.to_owned(), keypair.to_base58_string().into());
        props.insert(NAME_MARKER.to_owned(), name.into());

        let (_, node_id) = self.db.create_node((self.wallet_graph, props)).await?;

        Ok(node_id)
    }

    async fn remove_keypair(&self, node_id: NodeId) -> Result<Keypair, Error> {
        let keypair = self.get_keypair(node_id).await?;

        self.db.delete_node(node_id, self.wallet_graph).await?;

        Ok(keypair)
    }

    async fn get_keypair(&self, node_id: NodeId) -> Result<Keypair, Error> {
        let node = self.db.read_node(node_id).await?;

        let keypair = node
            .properties
            .get(KEYPAIR_MARKER)
            .ok_or(Error::KeypairDoesntExist)?
            .as_str()
            .unwrap();

        let keypair = Keypair::from_base58_string(keypair);

        Ok(keypair)
    }

    async fn insert_pubkey(&self, name: String, pubkey: Pubkey) -> Result<NodeId, Error> {
        let graph = self.db.read_graph(self.wallet_graph).await?;

        Self::check_name(&graph, &name)?;

        let mut props = Properties::default();

        props.insert(PUBKEY_MARKER.to_owned(), pubkey.to_string().into());
        props.insert(NAME_MARKER.to_owned(), name.into());

        let (_, node_id) = self.db.create_node((self.wallet_graph, props)).await?;

        Ok(node_id)
    }

    async fn remove_pubkey(&self, node_id: NodeId) -> Result<Pubkey, Error> {
        let pubkey = self.get_pubkey(node_id).await?;

        self.db.delete_node(node_id, self.wallet_graph).await?;

        Ok(pubkey)
    }

    async fn get_pubkey(&self, node_id: NodeId) -> Result<Pubkey, Error> {
        match self.get_keypair(node_id).await {
            Ok(keypair) => return Ok(keypair.pubkey()),
            Err(Error::KeypairDoesntExist) => (),
            Err(e) => return Err(e),
        };

        let node = self.db.read_node(node_id).await?;

        let pubkey = node
            .properties
            .get(PUBKEY_MARKER)
            .ok_or(Error::PubkeyDoesntExist)?
            .as_str()
            .unwrap();

        let pubkey = Pubkey::from_str(pubkey).unwrap();

        Ok(pubkey)
    }
}

pub struct Command {
    ctx: Arc<Ctx>,
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
    // DeleteKeypair(delete_keypair::DeleteKeypair),
    // AddPubkey(add_pubkey::AddPubkey),
    // DeletePubkey(delete_pubkey::DeletePubkey),
    // CreateAccount(create_account::CreateAccount),
    GetBalance(get_balance::GetBalance),
    // CreateToken(create_token::CreateToken),
    // RequestAirdrop(request_airdrop::RequestAirdrop),
    // MintToken(mint_token::MintToken),
    // Transfer(transfer::Transfer),
}

impl Command {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, ValueType>,
    ) -> Result<HashMap<String, ValueType>, Error> {
        match &self.kind {
            Kind::GenerateKeypair(k) => k.run(self.ctx.clone(), inputs).await,
            // Kind::DeleteKeypair(k) => k.run(self.ctx.clone(), inputs).await,
            // Kind::AddPubkey(k) => k.run(self.ctx.clone(), inputs).await,
            // Kind::DeletePubkey(k) => k.run(self.ctx.clone(), inputs).await,
            // Kind::CreateAccount(k) => k.run(self.ctx.clone(), inputs).await,
            Kind::GetBalance(k) => k.run(self.ctx.clone(), inputs).await,
            // Kind::CreateToken(k) => k.run(self.ctx.clone(), inputs).await,
            // Kind::RequestAirdrop(k) => k.run(self.ctx.clone(), inputs).await,
            // Kind::MintToken(k) => k.run(self.ctx.clone(), inputs).await,
            // Kind::Transfer(k) => k.run(self.ctx.clone(), inputs).await,
        }
    }
}
