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

use tempdir::TempDir;

use arloader::Arweave;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArweaveNftUpload {
    pub fee_payer: Option<NodeId>,
    pub reward_mult: Option<f32>,
    pub arweave_key_path: Option<String>,
    pub metadata: Option<NftMetadata>,
    pub pay_with_solana: Option<bool>,
}

impl ArweaveNftUpload {
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

        let reward_mult = match self.reward_mult {
            Some(s) => s,
            None => match inputs.remove("reward_mult") {
                Some(Value::F32(s)) => s,
                _ => return Err(Error::ArgumentNotFound("reward_mult".to_string())),
            },
        };

        let arweave_key_path = match &self.arweave_key_path {
            Some(s) => s.clone(),
            None => match inputs.remove("arweave_key_path") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("arweave_key_path".to_string())),
            },
        };

        let mut metadata = match &self.metadata {
            Some(s) => s.clone(),
            None => match inputs.remove("metadata") {
                Some(Value::NftMetadata(s)) => s,
                _ => return Err(Error::ArgumentNotFound("metadata".to_string())),
            },
        };

        let pay_with_solana = match self.pay_with_solana {
            Some(b) => b,
            None => match inputs.remove("pay_with_solana") {
                Some(Value::Bool(b)) => b,
                Some(Value::Empty) => false,
                _ => return Err(Error::ArgumentNotFound("pay_with_solana".to_string())),
            },
        };

        let tmp_dir = TempDir::new("sunshine_solana_junk").unwrap();

        let metadata_file_path = format!(
            "{}/{}",
            tmp_dir.path().to_str().unwrap(),
            uuid::Uuid::new_v4()
        );

        let file_map: Arc<DashMap<String, String>> = Arc::new(DashMap::new());

        let upload_file_with_cache =
            |fee_payer: &Keypair, arweave_key_path: &str, file_path: &str| {
                let fee_payer = Keypair::from_base58_string(&fee_payer.to_base58_string());
                let arweave_key_path = arweave_key_path.to_owned();
                let file_path = file_path.to_owned();
                let file_map = file_map.clone();
                let solana_net = ctx.solana_net;
                async move {
                    if let Some(file_url) = file_map.get(&file_path) {
                        return Ok::<String, Error>(file_url.clone());
                    }

                    let file_url = upload_file(
                        pay_with_solana,
                        solana_net,
                        arweave_key_path,
                        fee_payer,
                        file_path.clone(),
                        reward_mult,
                    )
                    .await?;

                    file_map.insert(file_path, file_url.clone());

                    Ok(file_url)
                }
            };

        metadata.image =
            upload_file_with_cache(&fee_payer, &arweave_key_path, &metadata.image).await?;

        for file in metadata.properties.files.iter_mut() {
            file.uri = upload_file_with_cache(&fee_payer, &arweave_key_path, &file.uri).await?;
        }

        tokio::fs::write(&metadata_file_path, serde_json::to_vec(&metadata).unwrap()).await?;

        let metadata_url =
            upload_file_with_cache(&fee_payer, &arweave_key_path, &metadata_file_path).await?;

        let outputs = hashmap! {
            "metadata_url".to_owned()=> Value::String(metadata_url),
            "metadata".to_owned() => Value::NftMetadata(metadata),
            "fee_payer".to_owned() => Value::Keypair(fee_payer.into()),
        };

        Ok(outputs)
    }
}

async fn upload_file(
    pay_with_solana: bool,
    solana_net: SolanaNet,
    arweave_key_path: String,
    fee_payer: Keypair,
    file_path: String,
    reward_mult: f32,
) -> Result<String, Error> {
    let (arweave, mut status) = if solana_net == SolanaNet::Mainnet || pay_with_solana {
        let arweave = Arweave {
            name: String::from("arweave"),
            units: String::from("sol"),
            base_url: url::Url::parse("https://arweave.net/").unwrap(),
            crypto: arloader::crypto::Provider::from_keypair_path(arweave_key_path.into()).await?,
        };

        let price_terms = arweave.get_price_terms(reward_mult).await?;

        let status = arweave
            .upload_file_from_path_with_sol(
                file_path.into(),
                None,
                None,
                None,
                price_terms,
                SolanaNet::Mainnet.url(),
                url::Url::parse("https://arloader.io/sol").unwrap(),
                &fee_payer,
            )
            .await?;

        (arweave, status)
    } else {
        let arweave = Arweave {
            name: String::from("arweave"),
            units: String::from("winstons"),
            base_url: url::Url::parse("https://arweave.net/").unwrap(),
            crypto: arloader::crypto::Provider::from_keypair_path(arweave_key_path.into()).await?,
        };

        let price_terms = arweave.get_price_terms(reward_mult).await?;

        let status = arweave
            .upload_file_from_path(file_path.into(), None, None, None, price_terms)
            .await?;

        (arweave, status)
    };

    loop {
        match status.status {
            StatusCode::Confirmed => break,
            StatusCode::NotFound => return Err(Error::ArweaveTxNotFound(status.id.to_string())),
            StatusCode::Submitted | StatusCode::Pending => {
                tokio::time::sleep(Duration::from_secs(1)).await;
                status = arweave.get_status(&status.id).await?;
            }
        }
    }

    Ok(format!("https://arweave.net/{}", status.id.to_string()))
}
