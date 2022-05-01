use std::path::PathBuf;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};

use super::super::Ctx;
use super::arweave_nft_upload::Uploader;
use dashmap::DashMap;
use maplit::hashmap;
use mpl_token_metadata::state::{Collection, Creator, UseMethod, Uses};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::keypair::Keypair, signer::Signer};
use spl_token::instruction::transfer_checked;
use uuid::Uuid;

use sunshine_core::msg::NodeId;

use crate::commands::solana::instructions::execute;
use crate::commands::solana::SolanaNet;
use crate::{Error, NftMetadata, Value};

use solana_sdk::signer::keypair::write_keypair_file;

use bundlr_sdk::{tags::Tag, Bundlr, Signer as BundlrSigner, SolanaSigner};

use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArweaveFileUpload {
    pub fee_payer: Option<NodeId>,
    pub file_path: Option<String>,
    pub fund_bundlr: Option<bool>,
}

impl ArweaveFileUpload {
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

        let file_path = match &self.file_path {
            Some(s) => s.clone(),
            None => match inputs.remove("file_path") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("file_path".to_string())),
            },
        };

        let fund_bundlr = match self.fund_bundlr {
            Some(b) => b,
            None => match inputs.remove("fund_bundlr") {
                Some(Value::Bool(b)) => b,
                Some(Value::Empty) => true,
                None => true,
                _ => return Err(Error::ArgumentNotFound("fund_bundlr".to_string())),
            },
        };

        let mut uploader = Uploader::new(ctx.solana_net, &fee_payer, ctx.clone())?;

        if fund_bundlr {
            uploader.lazy_fund(&file_path).await?;
        }

        let file_url = uploader.upload_file(&file_path).await?;

        let outputs = hashmap! {
            "file_url".to_owned()=> Value::String(file_url),
            "fee_payer".to_owned() => Value::Keypair(fee_payer.into()),
        };

        Ok(outputs)
    }
}
