use std::{collections::HashMap, sync::Arc};

use super::super::Ctx;
use borsh::BorshDeserialize;
use maplit::hashmap;
use mpl_token_metadata::state::MasterEditionV2;
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signer::Signer};

use sunshine_core::msg::NodeId;

use crate::{commands::solana::instructions::execute, CommandResult, Error, NftCreator, Value};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VerifyCollection {
    pub mint_account: Option<NodeId>,
    pub fee_payer: Option<NodeId>, // keypair
    pub collection_authority: Option<NodeId>,
    pub collection_mint_account: Option<NodeId>,
    pub collection_authority_is_delegated: Option<bool>,
}

impl VerifyCollection {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let mint_account = match self.mint_account {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("mint_account") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("mint_account".to_string())),
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

        let collection_authority = match self.collection_authority {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("collection_authority") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("collection_authority".to_string())),
            },
        };

        let collection_mint_account = match self.collection_mint_account {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("collection_mint_account") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => {
                    return Err(Error::ArgumentNotFound(
                        "collection_mint_account".to_string(),
                    ))
                }
            },
        };

        let collection_authority_is_delegated = match self.collection_authority_is_delegated {
            Some(s) => s,
            None => match inputs.remove("collection_authority_is_delegated") {
                Some(Value::Bool(s)) => s,
                None => false,
                _ => {
                    return Err(Error::ArgumentNotFound(
                        "collection_authority_is_delegated".to_string(),
                    ))
                }
            },
        };

        let (collection_metadata_account, _) =
            mpl_token_metadata::pda::find_metadata_account(&collection_mint_account);

        let (collection_master_edition_account, _) =
            mpl_token_metadata::pda::find_master_edition_account(&collection_mint_account);

        let collection_authority_record = if collection_authority_is_delegated {
            Some(
                mpl_token_metadata::pda::find_collection_authority_account(
                    &mint_account,
                    &collection_authority.pubkey(),
                )
                .0,
            )
        } else {
            None
        };

        let (metadata_account, _) = mpl_token_metadata::pda::find_metadata_account(&mint_account);

        let (minimum_balance_for_rent_exemption, instructions) = command_verify_collection(
            metadata_account,
            collection_authority.pubkey(),
            fee_payer.pubkey(),
            collection_mint_account,
            collection_metadata_account,
            collection_master_edition_account,
            collection_authority_record,
        )?;

        let fee_payer_pubkey = fee_payer.pubkey();

        let signers: Vec<&dyn Signer> = vec![&collection_authority, &fee_payer];

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
            "fee_payer".to_owned() => Value::Keypair(fee_payer.into()),
            "mint_account".to_owned() => Value::Pubkey(mint_account.into()),
            "collection_authority".to_owned() => Value::Keypair(collection_authority.into()),
        };

        Ok(outputs)
    }
}

pub fn command_verify_collection(
    metadata: Pubkey,
    collection_authority: Pubkey,
    payer: Pubkey,
    collection_mint: Pubkey,
    collection: Pubkey,
    collection_master_edition_account: Pubkey,
    collection_authority_record: Option<Pubkey>,
) -> CommandResult {
    let instructions = vec![mpl_token_metadata::instruction::verify_collection(
        mpl_token_metadata::id(),
        metadata,
        collection_authority,
        payer,
        collection_mint,
        collection,
        collection_master_edition_account,
        collection_authority_record,
    )];

    Ok((0, instructions))
}
