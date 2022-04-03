use std::{collections::HashMap, sync::Arc};

use super::super::Ctx;
use maplit::hashmap;
use mpl_token_metadata::state::{Collection, Creator, UseMethod, Uses};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::keypair::Keypair, signer::Signer};

use sunshine_core::msg::NodeId;

use crate::{commands::solana::instructions::execute, CommandResult, Error, NftCreator, Value};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApproveCollectionAuthority {
    pub new_collection_authority: Option<NodeId>,
    pub update_authority: Option<NodeId>,
    pub fee_payer: Option<NodeId>, // keypair
    pub mint_account: Option<NodeId>,
}

impl ApproveCollectionAuthority {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let new_collection_authority = match self.new_collection_authority {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("new_collection_authority") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => {
                    return Err(Error::ArgumentNotFound(
                        "new_collection_authority".to_string(),
                    ))
                }
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

        let fee_payer = match self.fee_payer {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("fee_payer") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("fee_payer".to_string())),
            },
        };

        let mint_account = match self.mint_account {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("mint_account") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("mint_account".to_string())),
            },
        };

        let program_id = mpl_token_metadata::id();

        let metadata_seeds = &[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            &program_id.as_ref(),
            mint_account.as_ref(),
        ];

        let (metadata_pubkey, _) = Pubkey::find_program_address(metadata_seeds, &program_id);

        let (collection_authority_record, _) =
            mpl_token_metadata::pda::find_collection_authority_account(
                &mint_account,
                &new_collection_authority,
            );

        let (minimum_balance_for_rent_exemption, instructions) =
            command_approve_collection_authority(
                &ctx.client,
                collection_authority_record,
                new_collection_authority,
                update_authority.pubkey(),
                fee_payer.pubkey(),
                metadata_pubkey,
                mint_account,
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
            "fee_payer".to_owned() => Value::Keypair(fee_payer.into()),
            "update_authority".to_owned() => Value::Keypair(update_authority.into()),
            "mint_account".to_owned() => Value::Pubkey(mint_account.into()),
        };

        Ok(outputs)
    }
}

pub fn command_approve_collection_authority(
    rpc_client: &RpcClient,
    collection_authority_record: Pubkey,
    new_collection_authority: Pubkey,
    update_authority: Pubkey,
    payer: Pubkey,
    metadata: Pubkey,
    mint: Pubkey,
) -> CommandResult {
    let minimum_balance_for_rent_exemption =
        rpc_client.get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            mpl_token_metadata::state::CollectionAuthorityRecord,
        >())?;

    let instructions = vec![
        mpl_token_metadata::instruction::approve_collection_authority(
            mpl_token_metadata::id(),
            collection_authority_record,
            new_collection_authority,
            update_authority,
            payer,
            metadata,
            mint,
        ),
    ];

    Ok((minimum_balance_for_rent_exemption, instructions))
}
