use std::path::PathBuf;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};

use super::super::Ctx;
use arloader::crypto::Provider;
use dashmap::DashMap;
use maplit::hashmap;
use mpl_token_metadata::state::{Collection, Creator, UseMethod, Uses};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::keypair::Keypair, signer::Signer};
use uuid::Uuid;

use arloader::status::OutputFormat;
use arloader::{commands::command_upload_nfts, status::StatusCode};

use sunshine_core::msg::NodeId;

use crate::commands::solana::SolanaNet;
use crate::{Error, NftMetadata, Value};

use solana_sdk::signer::keypair::write_keypair_file;

use bundlr_sdk::{tags::Tag, Bundlr, Signer as BundlrSigner, SolanaSigner};

use arloader::Arweave;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArweaveBundlr {
    pub fee_payer: Option<NodeId>,
    pub metadata: Option<NftMetadata>,
}

impl ArweaveBundlr {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let fee_payer = match self.fee_payer {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("fee_payer") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("fee_payer".to_string())),
            },
        };

        let mut metadata = match &self.metadata {
            Some(s) => s.clone(),
            None => match inputs.remove("metadata") {
                Some(Value::NftMetadata(s)) => s,
                _ => return Err(Error::ArgumentNotFound("metadata".to_string())),
            },
        };

        let mut uploader = Uploader::new(ctx.solana_net, &fee_payer)?;

        metadata.image = uploader.upload_file(&metadata.image).await?;

        for file in metadata.properties.files.iter_mut() {
            file.uri = uploader.upload_file(&file.uri).await?;
        }

        let metadata_url = uploader
            .upload(
                serde_json::to_vec(&metadata).unwrap(),
                "application/json".to_owned(),
            )
            .await?;

        let outputs = hashmap! {
            "metadata_url".to_owned()=> Value::String(metadata_url),
            "metadata".to_owned() => Value::NftMetadata(metadata),
            "fee_payer".to_owned() => Value::Keypair(fee_payer.into()),
        };

        Ok(outputs)
    }
}

struct Uploader {
    cache: HashMap<String, String>,
    signer: String,
    node_url: String,
}

impl Uploader {
    fn new(solana_net: SolanaNet, fee_payer: &Keypair) -> Result<Uploader, Error> {
        let node_url = match solana_net {
            SolanaNet::Mainnet => "https://node1.bundlr.network".to_owned(),
            SolanaNet::Devnet => "https://devnet.bundlr.network".to_owned(),
            SolanaNet::Testnet => return Err(Error::BundlrNotAvailableOnTestnet),
        };

        Ok(Uploader {
            cache: HashMap::new(),
            signer: fee_payer.to_base58_string(),
            node_url,
        })
    }

    async fn upload_file(&mut self, file_path: &str) -> Result<String, Error> {
        if let Some(url) = self.cache.get(file_path) {
            return Ok(url.clone());
        }

        let content_type = mime_guess::from_path(file_path)
            .first()
            .ok_or(Error::MimeTypeNotFound)?
            .to_string();
        let data = tokio::fs::read(file_path).await?;

        let url = self.upload(data, content_type).await?;

        self.cache.insert(file_path.to_owned(), url.clone());

        Ok(url)
    }

    async fn upload(&self, data: Vec<u8>, content_type: String) -> Result<String, Error> {
        let bundlr = Bundlr::new(
            self.node_url.clone(),
            "solana".to_string(),
            "sol".to_string(),
            SolanaSigner::from_base58(&self.signer),
        );
        let tx = bundlr.create_transaction_with_tags(
            data,
            vec![Tag::new("Content-Type".into(), content_type)],
        );

        let resp: BundlrResponse = serde_json::from_value(bundlr.send_transaction(tx).await?)?;

        Ok(format!("https://arweave.net/{}", resp.id))
    }
}

#[derive(Deserialize)]
struct BundlrResponse {
    id: String,
}
