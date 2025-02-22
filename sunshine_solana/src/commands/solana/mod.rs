use std::{collections::HashMap, str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use sunshine_core::msg::{Graph, GraphId, Properties};
use sunshine_core::store::Datastore;
use url::Url;

use crate::{error::Error, Value};

use sunshine_core::msg::NodeId;

mod instructions;

pub mod add_pubkey;
pub mod create_mint_account;
pub mod create_token_account;
pub mod delete_keypair;
pub mod delete_pubkey;
pub mod generate_keypair;
pub mod get_balance;
pub mod mint_token;
pub mod nft;
pub mod request_airdrop;
pub mod transfer_solana;
pub mod transfer_token;

const KEYPAIR_MARKER: &str = "KEYPAIR_MARKER";
const NAME_MARKER: &str = "NAME_MARKER";
const PUBKEY_MARKER: &str = "PUBKEY_MARKER";

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub solana_net: SolanaNet,
    pub wallet_graph: GraphId,
}

pub struct Ctx {
    client: RpcClient,
    db: Arc<dyn Datastore>,
    wallet_graph: GraphId,
    solana_net: SolanaNet,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
pub enum SolanaNet {
    Devnet,
    Testnet,
    Mainnet,
}

impl SolanaNet {
    pub fn url(&self) -> Url {
        let solana_url = match self {
            SolanaNet::Devnet => "https://api.devnet.solana.com",
            SolanaNet::Testnet => "https://api.testnet.solana.com",
            SolanaNet::Mainnet => "https://api.mainnet-beta.solana.com",
        };

        Url::parse(solana_url).unwrap()
    }
}

impl Ctx {
    pub fn new(cfg: Config, db: Arc<dyn Datastore>) -> Result<Ctx, Error> {
        Ok(Ctx {
            client: RpcClient::new(cfg.solana_net.url()),
            wallet_graph: cfg.wallet_graph,
            db,
            solana_net: cfg.solana_net,
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

        props.insert(NAME_MARKER.to_owned(), name.into());
        props.insert(KEYPAIR_MARKER.to_owned(), keypair.to_base58_string().into());

        let (_, node_id) = self.db.create_node((self.wallet_graph, props)).await?;

        Ok(node_id)
    }

    async fn remove_keypair(&self, node_id: NodeId) -> Result<Keypair, Error> {
        let keypair = self.get_keypair_by_id(node_id).await?;

        self.db.delete_node(node_id, self.wallet_graph).await?;

        Ok(keypair)
    }

    async fn get_keypair_by_id(&self, node_id: NodeId) -> Result<Keypair, Error> {
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

    async fn get_node_id_by_keypair(&self, input_keypair: &str) -> Result<NodeId, Error> {
        let graph = self.db.read_graph(self.wallet_graph).await?;

        let node_id = *graph
            .nodes
            .iter()
            .filter(|&node| {
                let keypair = node.properties.get(KEYPAIR_MARKER).unwrap();
                keypair == input_keypair
            })
            .map(|node| node.node_id)
            .collect::<Vec<NodeId>>()
            .first()
            .unwrap();

        Ok(node_id)
    }

    async fn insert_pubkey(&self, name: String, pubkey: Pubkey) -> Result<NodeId, Error> {
        let graph = self.db.read_graph(self.wallet_graph).await?;

        Self::check_name(&graph, &name)?;

        let mut props = Properties::default();

        props.insert(NAME_MARKER.to_owned(), name.into());
        props.insert(PUBKEY_MARKER.to_owned(), pubkey.to_string().into());

        let (_, node_id) = self.db.create_node((self.wallet_graph, props)).await?;

        Ok(node_id)
    }

    async fn remove_pubkey(&self, node_id: NodeId) -> Result<Pubkey, Error> {
        let pubkey = self.get_pubkey_by_id(node_id).await?;

        self.db.delete_node(node_id, self.wallet_graph).await?;

        Ok(pubkey)
    }

    async fn get_node_id_by_pubkey(&self, input_pubkey: Pubkey) -> Result<NodeId, Error> {
        let graph = self.db.read_graph(self.wallet_graph).await?;

        let node_id = *graph
            .nodes
            .iter()
            .filter(|&node| {
                let pubkey = node
                    .properties
                    .get(PUBKEY_MARKER)
                    .ok_or(Error::PubkeyDoesntExist)
                    .unwrap()
                    .as_str()
                    .unwrap();
                pubkey == Pubkey::to_string(&input_pubkey)
            })
            .map(|node| node.node_id)
            .collect::<Vec<NodeId>>()
            .first()
            .unwrap();

        Ok(node_id)
    }

    async fn get_pubkey_by_id(&self, node_id: NodeId) -> Result<Pubkey, Error> {
        match self.get_keypair_by_id(node_id).await {
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
    pub ctx: Arc<Ctx>,
    pub kind: Kind,
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
    CreateTokenAccount(create_token_account::CreateTokenAccount),
    GetBalance(get_balance::GetBalance),
    CreateMintAccount(create_mint_account::CreateMintAccount),
    RequestAirdrop(request_airdrop::RequestAirdrop),
    MintToken(mint_token::MintToken),
    TransferToken(transfer_token::TransferToken),
    Nft(nft::Command),
    TransferSolana(transfer_solana::TransferSolana),
}

impl Kind {
    pub fn kind(&self) -> CommandKind {
        match &self {
            Kind::GenerateKeypair(_) => CommandKind::GenerateKeypair,
            Kind::DeleteKeypair(_) => CommandKind::DeleteKeypair,
            Kind::AddPubkey(_) => CommandKind::AddPubkey,
            Kind::DeletePubkey(_) => CommandKind::DeletePubkey,
            Kind::CreateTokenAccount(_) => CommandKind::CreateTokenAccount,
            Kind::GetBalance(_) => CommandKind::GetBalance,
            Kind::CreateMintAccount(_) => CommandKind::CreateMintAccount,
            Kind::RequestAirdrop(_) => CommandKind::RequestAirdrop,
            Kind::MintToken(_) => CommandKind::MintToken,
            Kind::TransferToken(_) => CommandKind::TransferToken,
            Kind::Nft(n) => CommandKind::Nft(n.kind()),
            Kind::TransferSolana(_) => CommandKind::TransferSolana,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CommandKind {
    GenerateKeypair,
    DeleteKeypair,
    AddPubkey,
    DeletePubkey,
    CreateTokenAccount,
    GetBalance,
    CreateMintAccount,
    RequestAirdrop,
    MintToken,
    TransferToken,
    Nft(nft::CommandKind),
    TransferSolana,
}

impl Command {
    pub(crate) async fn run(
        &self,
        inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        match &self.kind {
            Kind::GenerateKeypair(k) => k.run(self.ctx.clone(), inputs).await,
            Kind::DeleteKeypair(k) => k.run(self.ctx.clone(), inputs).await,
            Kind::AddPubkey(k) => k.run(self.ctx.clone(), inputs).await,
            Kind::DeletePubkey(k) => k.run(self.ctx.clone(), inputs).await,
            Kind::CreateTokenAccount(k) => k.run(self.ctx.clone(), inputs).await,
            Kind::GetBalance(k) => k.run(self.ctx.clone(), inputs).await,
            Kind::CreateMintAccount(k) => k.run(self.ctx.clone(), inputs).await,
            Kind::RequestAirdrop(k) => k.run(self.ctx.clone(), inputs).await,
            Kind::MintToken(k) => k.run(self.ctx.clone(), inputs).await,
            Kind::TransferToken(k) => k.run(self.ctx.clone(), inputs).await,
            Kind::Nft(k) => k.run(self.ctx.clone(), inputs).await,
            Kind::TransferSolana(k) => k.run(self.ctx.clone(), inputs).await,
        }
    }

    pub fn kind(&self) -> CommandKind {
        match &self.kind {
            Kind::GenerateKeypair(_) => CommandKind::GenerateKeypair,
            Kind::DeleteKeypair(_) => CommandKind::DeleteKeypair,
            Kind::AddPubkey(_) => CommandKind::AddPubkey,
            Kind::DeletePubkey(_) => CommandKind::DeletePubkey,
            Kind::CreateTokenAccount(_) => CommandKind::CreateTokenAccount,
            Kind::GetBalance(_) => CommandKind::GetBalance,
            Kind::CreateMintAccount(_) => CommandKind::CreateMintAccount,
            Kind::RequestAirdrop(_) => CommandKind::RequestAirdrop,
            Kind::MintToken(_) => CommandKind::MintToken,
            Kind::TransferToken(_) => CommandKind::TransferToken,
            Kind::Nft(n) => CommandKind::Nft(n.kind()),
            Kind::TransferSolana(_) => CommandKind::TransferSolana,
        }
    }
}
