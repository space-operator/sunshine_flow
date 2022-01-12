use std::{collections::HashMap, sync::Arc};

use super::super::Ctx;
use maplit::hashmap;
use mpl_token_metadata::state::{Creator, DataV2};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

use sunshine_core::msg::NodeId;

use super::create_metadata_accounts::{NftCollection, NftUses};
use crate::{commands::solana::instructions::execute, CommandResult, Error, NftCreator, Value};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateMetadataAccounts {
    pub token: Option<NodeId>,
    pub metadata_pubkey: Option<NodeId>,
    pub fee_payer: Option<NodeId>,                    // keypair
    pub update_authority: Option<NodeId>,             // keypair
    pub new_update_authority: Option<Option<NodeId>>, // keypair
    pub data: Option<Option<MetadataAccountData>>,
    pub primary_sale_happened: Option<Option<bool>>,
    pub is_mutable: Option<Option<bool>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetadataAccountData {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub seller_fee_basis_points: u16,
    pub creators: Option<Vec<NftCreator>>,
    pub collection: Option<NftCollection>,
    pub uses: Option<NftUses>,
}

impl Into<DataV2> for MetadataAccountData {
    fn into(self) -> DataV2 {
        DataV2 {
            name: self.name,
            symbol: self.symbol,
            uri: self.uri,
            seller_fee_basis_points: self.seller_fee_basis_points,
            creators: self
                .creators
                .map(|c| c.into_iter().map(Into::into).collect()),
            collection: self.collection.map(Into::into),
            uses: self.uses.map(Into::into),
        }
    }
}

impl UpdateMetadataAccounts {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let token = match self.token {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("token") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("token".to_string())),
            },
        };

        let metadata_pubkey = match self.metadata_pubkey {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("metadata_pubkey") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("metadata_pubkey".to_string())),
            },
        };

        let fee_payer = match self.fee_payer {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("fee_payer") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("fee_payer".to_string())),
            },
        };

        let update_authority = match self.update_authority {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("update_authority") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("update_authority".to_string())),
            },
        };

        let new_update_authority = match self.new_update_authority {
            Some(s) => match s {
                Some(s) => Some(ctx.get_pubkey_by_id(s).await?),
                None => None,
            },
            None => match inputs.remove("new_update_authority") {
                Some(Value::NodeId(s)) => Some(ctx.get_pubkey_by_id(s).await?),
                Some(Value::Pubkey(k)) => Some(k),
                Some(Value::Empty) => None,
                _ => return Err(Error::ArgumentNotFound("new_update_authority".to_string())),
            },
        };

        let data: Option<DataV2> = match self.data.clone() {
            Some(data) => data.map(Into::into),
            None => match inputs.remove("data") {
                Some(Value::MetadataAccountData(data)) => Some(data.into()),
                Some(Value::Empty) => None,
                _ => return Err(Error::ArgumentNotFound("data".to_string())),
            },
        };

        let primary_sale_happened = match self.primary_sale_happened {
            Some(s) => s,
            None => match inputs.remove("primary_sale_happened") {
                Some(Value::Bool(s)) => Some(s),
                Some(Value::Empty) => None,
                _ => return Err(Error::ArgumentNotFound("primary_sale_happened".to_string())),
            },
        };

        let is_mutable = match self.is_mutable {
            Some(s) => s,
            None => match inputs.remove("is_mutable") {
                Some(Value::Bool(s)) => Some(s),
                Some(Value::Empty) => None,
                _ => return Err(Error::ArgumentNotFound("is_mutable".to_string())),
            },
        };

        let program_id = mpl_token_metadata::id();

        let metadata_seeds = &[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            &program_id.as_ref(),
            token.as_ref(),
        ];

        let (metadata_pubkey, _) = Pubkey::find_program_address(metadata_seeds, &program_id);

        let (minimum_balance_for_rent_exemption, instructions) = command_update_metadata_accounts(
            &ctx.client,
            metadata_pubkey,
            update_authority.pubkey(),
            new_update_authority,
            data,
            primary_sale_happened,
            is_mutable,
        )?;

        let fee_payer_pubkey = fee_payer.pubkey();

        let signers: Vec<&dyn Signer> = vec![&update_authority, &fee_payer];

        let res = execute(
            &signers,
            &ctx.client,
            &fee_payer_pubkey,
            &instructions,
            minimum_balance_for_rent_exemption,
        );

        let signature = res?;

        let outputs = hashmap! {
            "signature".to_owned()=>Value::Success(signature),
            "fee_payer".to_owned()=>Value::Keypair(fee_payer.into()),
            "token".to_owned()=>Value::Pubkey(token),
            "metadata_pubkey".to_owned()=>Value::Pubkey(metadata_pubkey),
        };

        Ok(outputs)
    }
}

pub fn command_update_metadata_accounts(
    rpc_client: &RpcClient,
    metadata_pubkey: Pubkey,
    update_authority: Pubkey,
    new_update_authority: Option<Pubkey>,
    data: Option<DataV2>,
    primary_sale_happened: Option<bool>,
    is_mutable: Option<bool>,
) -> CommandResult {
    let instructions = vec![
        mpl_token_metadata::instruction::update_metadata_accounts_v2(
            mpl_token_metadata::id(),
            metadata_pubkey,
            update_authority,
            new_update_authority,
            data,
            primary_sale_happened,
            is_mutable,
        ),
    ];

    Ok((0, instructions))
}
