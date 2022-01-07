use std::{collections::HashMap, sync::Arc};

use super::super::Ctx;
use maplit::hashmap;
use metaplex_token_metadata::state::Creator;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

use sunshine_core::msg::NodeId;

use crate::{commands::solana::instructions::execute, CommandResult, Error, NftCreator, Value};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateMetadataAccounts {
    metadata_account: Option<NodeId>,
    token: Option<NodeId>,
    token_authority: Option<NodeId>,
    fee_payer: Option<NodeId>,        // keypair
    update_authority: Option<NodeId>, // keypair
    name: Option<String>,
    symbol: Option<String>,
    uri: Option<String>,
    creators: Option<Vec<NftCreator>>,
    seller_fee_basis_points: Option<u16>,
    update_authority_is_signer: Option<bool>,
    is_mutable: Option<bool>,
}

impl CreateMetadataAccounts {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let metadata_account = match self.metadata_account {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("metadata_account") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("metadata_account".to_string())),
            },
        };

        let token = match self.token {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("token") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("token".to_string())),
            },
        };

        let token_authority = match self.token_authority {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("token_authority") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("token_authority".to_string())),
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

        let name = match &self.name {
            Some(s) => s.clone(),
            None => match inputs.remove("name") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("name".to_string())),
            },
        };

        let symbol = match &self.symbol {
            Some(s) => s.clone(),
            None => match inputs.remove("symbol") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("symbol".to_string())),
            },
        };

        let uri = match &self.uri {
            Some(s) => s.clone(),
            None => match inputs.remove("uri") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("uri".to_string())),
            },
        };

        let creators = match self.creators.clone() {
            Some(s) => s,
            None => match inputs.remove("creators") {
                Some(Value::NftCreators(s)) => s,
                _ => return Err(Error::ArgumentNotFound("creators".to_string())),
            },
        };

        let seller_fee_basis_points = match self.seller_fee_basis_points {
            Some(s) => s,
            None => match inputs.remove("seller_fee_basis_points") {
                Some(Value::U16(s)) => s,
                _ => {
                    return Err(Error::ArgumentNotFound(
                        "seller_fee_basis_points".to_string(),
                    ))
                }
            },
        };

        let update_authority_is_signer = match self.update_authority_is_signer {
            Some(s) => s,
            None => match inputs.remove("update_authority_is_signer") {
                Some(Value::Bool(s)) => s,
                _ => {
                    return Err(Error::ArgumentNotFound(
                        "update_authority_is_signer".to_string(),
                    ))
                }
            },
        };

        let is_mutable = match self.is_mutable {
            Some(s) => s,
            None => match inputs.remove("is_mutable") {
                Some(Value::Bool(s)) => s,
                _ => return Err(Error::ArgumentNotFound("is_mutable".to_string())),
            },
        };

        let creators = if creators.is_empty() {
            None
        } else {
            Some(creators.into_iter().map(NftCreator::into).collect())
        };

        let (minimum_balance_for_rent_exemption, instructions) = command_create_metadata_accounts(
            &ctx.client,
            metadata_account,
            token,
            token_authority,
            fee_payer.pubkey(),
            update_authority.pubkey(),
            name,
            symbol,
            uri,
            creators,
            seller_fee_basis_points,
            update_authority_is_signer,
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
        };

        Ok(outputs)
    }
}

// minting account - metadata
//        |
//  nft token

pub fn command_create_metadata_accounts(
    rpc_client: &RpcClient,
    metadata_account: Pubkey,
    mint: Pubkey,
    mint_authority: Pubkey,
    payer: Pubkey,
    update_authority: Pubkey,
    name: String,
    symbol: String,
    uri: String,
    creators: Option<Vec<Creator>>,
    seller_fee_basis_points: u16,
    update_authority_is_signer: bool,
    is_mutable: bool,
) -> CommandResult {
    let minimum_balance_for_rent_exemption =
        rpc_client.get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            metaplex_token_metadata::state::Metadata,
        >())?;

    let instructions = vec![
        metaplex_token_metadata::instruction::create_metadata_accounts(
            metaplex_token_metadata::id(),
            metadata_account,
            mint,
            mint_authority,
            payer,
            update_authority,
            name,
            symbol,
            uri,
            creators,
            seller_fee_basis_points,
            update_authority_is_signer,
            is_mutable,
        ),
    ];

    Ok((minimum_balance_for_rent_exemption, instructions))
}
