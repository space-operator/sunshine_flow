use std::path::PathBuf;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};

use super::super::Ctx;
use arloader::crypto::Provider;
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
use crate::{Error, Value};

use solana_sdk::signer::keypair::write_keypair_file;

use arloader::Arweave;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArweaveNftUpload {
    pub fee_payer: Option<NodeId>,
    pub reward_mult: Option<f32>,
    pub file_path: Option<String>,
    pub arweave_key_path: Option<String>,
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

        let file_path = match &self.file_path {
            Some(s) => s.clone(),
            None => match inputs.remove("file_path") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("file_path".to_string())),
            },
        };

        let arweave_key_path = match &self.arweave_key_path {
            Some(s) => s.clone(),
            None => match inputs.remove("arweave_key_path") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("arweave_key_path".to_string())),
            },
        };

        let (arweave, mut status) = match ctx.solana_net {
            SolanaNet::Mainnet => {
                let arweave = Arweave {
                    name: String::from("arweave"),
                    units: String::from("sol"),
                    base_url: url::Url::parse("https://arweave.net/").unwrap(),
                    crypto: arloader::crypto::Provider::from_keypair_path(arweave_key_path.into())
                        .await?,
                };

                let price_terms = arweave.get_price_terms(reward_mult).await?;

                let status = arweave
                    .upload_file_from_path_with_sol(
                        file_path.into(),
                        None,
                        None,
                        None,
                        price_terms,
                        ctx.solana_net.url(),
                        url::Url::parse("https://arloader.io/sol").unwrap(),
                        &fee_payer,
                    )
                    .await?;

                (arweave, status)
            }
            _ => {
                let arweave = Arweave {
                    name: String::from("arweave"),
                    units: String::from("winstons"),
                    base_url: url::Url::parse("https://arweave.net/").unwrap(),
                    crypto: arloader::crypto::Provider::from_keypair_path(arweave_key_path.into())
                        .await?,
                };

                let price_terms = arweave.get_price_terms(reward_mult).await?;

                let status = arweave
                    .upload_file_from_path(file_path.into(), None, None, None, price_terms)
                    .await?;

                (arweave, status)
            }
        };

        loop {
            match status.status {
                StatusCode::Confirmed => break,
                StatusCode::NotFound => {
                    return Err(Error::ArweaveTxNotFound(status.id.to_string()))
                }
                StatusCode::Submitted | StatusCode::Pending => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    status = arweave.get_status(&status.id).await?;
                }
            }
        }

        let outputs = hashmap! {
            "fee_payer".to_owned() => Value::Keypair(fee_payer.into()),
            "file_uri".to_owned() => Value::String(format!("https://arweave.net/{}", status.id.to_string())),
        };

        Ok(outputs)
    }
}
