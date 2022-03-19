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
use spl_token::instruction::transfer_checked;
use uuid::Uuid;

use arloader::status::OutputFormat;
use arloader::{commands::command_upload_nfts, status::StatusCode};

use sunshine_core::msg::NodeId;

use crate::commands::solana::instructions::execute;
use crate::commands::solana::SolanaNet;
use crate::{Error, NftMetadata, Value};

use solana_sdk::signer::keypair::write_keypair_file;

use bundlr_sdk::{tags::Tag, Bundlr, Signer as BundlrSigner, SolanaSigner};

use arloader::Arweave;

use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArweaveBundlr {
    pub fee_payer: Option<NodeId>,
    pub metadata: Option<NftMetadata>,
    pub fund_bundlr: Option<bool>,
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

        let fund_bundlr = match self.fund_bundlr {
            Some(b) => b,
            None => match inputs.remove("fund_bundlr") {
                Some(Value::Bool(b)) => b,
                Some(Value::Empty) => true,
                _ => return Err(Error::ArgumentNotFound("fund_bundlr".to_string())),
            },
        };

        let mut uploader = Uploader::new(ctx.solana_net, &fee_payer, ctx.clone())?;

        if fund_bundlr {
            uploader.lazy_fund(&metadata).await?;
        }

        metadata.image = uploader.upload_file(&metadata.image).await?;

        if let Some(properties) = metadata.properties.as_mut() {
            if let Some(files) = properties.files.as_mut() {
                for file in files.iter_mut() {
                    file.uri = uploader.upload_file(&file.uri).await?;
                }
            }
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
    fee_payer: String,
    node_url: String,
    ctx: Arc<Ctx>,
}

impl Uploader {
    fn new(solana_net: SolanaNet, fee_payer: &Keypair, ctx: Arc<Ctx>) -> Result<Uploader, Error> {
        let node_url = match solana_net {
            SolanaNet::Mainnet => "https://node1.bundlr.network".to_owned(),
            SolanaNet::Devnet => "https://devnet.bundlr.network".to_owned(),
            SolanaNet::Testnet => return Err(Error::BundlrNotAvailableOnTestnet),
        };

        Ok(Uploader {
            cache: HashMap::new(),
            fee_payer: fee_payer.to_base58_string(),
            node_url,
            ctx,
        })
    }

    async fn lazy_fund(&self, metadata: &NftMetadata) -> Result<(), Error> {
        use std::collections::HashSet;

        let mut processed = HashSet::new();
        let mut needed_size = 0;

        let metadata_size = serde_json::to_vec(metadata).unwrap().len() as u64;

        needed_size += metadata_size;
        needed_size += Self::get_file_size(&metadata.image).await?;
        processed.insert(metadata.image.clone());

        if let Some(properties) = metadata.properties.as_ref() {
            if let Some(files) = properties.files.as_ref() {
                for file in files.iter() {
                    if processed.contains(&file.uri) {
                        continue;
                    }

                    needed_size += Self::get_file_size(&file.uri).await?;
                    processed.insert(file.uri.clone());
                }
            }
        }

        needed_size += 100_000; // tx_fee + some offset
        needed_size += metadata_size * 4 / 10; // metadata change offset

        let needed_balance = self.get_price(needed_size).await?;
        let needed_balance = needed_balance + needed_balance / 10;

        let current_balance = self.get_current_balance().await?;

        if current_balance < needed_balance {
            self.fund(needed_balance - current_balance).await?;
        }

        Ok(())
    }

    async fn get_file_size(path: &str) -> Result<u64, Error> {
        let file = tokio::fs::File::open(path).await?;

        Ok(file.metadata().await?.len())
    }

    async fn get_price(&self, size: u64) -> Result<u64, Error> {
        let resp = reqwest::get(format!("{}/price/solana/{}", &self.node_url, size,)).await?;

        Ok(u64::from_str(&resp.text().await?).map_err(|_| Error::BundlrApiInvalidResponse)?)
    }

    async fn get_current_balance(&self) -> Result<u64, Error> {
        #[derive(Deserialize, Serialize)]
        struct Resp {
            balance: String,
        }

        let keypair = Keypair::from_base58_string(&self.fee_payer);

        let resp = reqwest::get(format!(
            "{}/account/balance/solana/?address={}",
            &self.node_url,
            keypair.pubkey()
        ))
        .await?;

        let resp: Resp = serde_json::from_str(&resp.text().await?)?;

        Ok(u64::from_str(&resp.balance).map_err(|_| Error::BundlrApiInvalidResponse)?)
    }

    async fn fund(&self, amount: u64) -> Result<(), Error> {
        #[derive(Deserialize, Serialize)]
        struct Addresses {
            solana: String,
        }

        #[derive(Deserialize, Serialize)]
        struct Info {
            addresses: Addresses,
        }

        let resp = reqwest::get(format!("{}/info", &self.node_url)).await?;

        let info: Info = serde_json::from_str(&resp.text().await?)?;

        let recipient = Pubkey::from_str(&info.addresses.solana)?;

        let fee_payer = Keypair::from_base58_string(&self.fee_payer);

        let recent_blockhash = self.ctx.client.get_latest_blockhash()?;

        let tx = solana_sdk::system_transaction::transfer(
            &fee_payer,
            &recipient,
            amount,
            recent_blockhash,
        );

        let signature = self.ctx.client.send_and_confirm_transaction(&tx)?;

        let resp = reqwest::Client::new()
            .post(format!("{}/account/balance/solana", &self.node_url))
            .json(&serde_json::json!({
                "tx_id": signature.to_string(),
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Error::BundlrTxRegisterFailed(signature.to_string()));
        }

        Ok(())
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
            SolanaSigner::from_base58(&self.fee_payer),
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

/*
H6RYSz54qPAMNKKWDqZa418NU3695DofXat8FDkxFcex
*/
