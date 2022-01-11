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
pub struct UpdateMetadataAccounts {
    pub metadata_pubkey: Option<NodeId>,
    pub fee_payer: Option<NodeId>,        // keypair
    pub update_authority: Option<NodeId>, // keypair
    pub new_update_authority: Option<Option<NodeId>>, // keypair
    pub primary_sale_happened: Option<Option<bool>>,
    pub data: Option<Option<MetadataAccountData>>,
}

pub struct MetadataAccountData {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub seller_fee_basis_points: u16,
    pub creators: Option<Vec<Creator>>,
}

impl CreateMetadataAccounts {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
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

        let program_id = metaplex_token_metadata::id();

        let metadata_seeds = &[
            metaplex_token_metadata::state::PREFIX.as_bytes(),
            &program_id.as_ref(),
            token.as_ref(),
        ];

        let (metadata_pubkey, _) = Pubkey::find_program_address(metadata_seeds, &program_id);

        let (minimum_balance_for_rent_exemption, instructions) = command_create_metadata_accounts(
            &ctx.client,
            metadata_pubkey,
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
            "fee_payer".to_owned()=>Value::Keypair(fee_payer.into()),
            "token".to_owned()=>Value::Pubkey(token),
            "metadata_pubkey".to_owned()=>Value::Pubkey(metadata_pubkey),
        };

        Ok(outputs)
    }
}

pub fn command_create_metadata_accounts(
    rpc_client: &RpcClient,
    metadata_pubkey: Pubkey,
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
            metadata_pubkey,
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
