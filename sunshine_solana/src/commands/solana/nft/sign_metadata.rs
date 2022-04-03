use std::{collections::HashMap, sync::Arc};

use super::super::Ctx;
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signer::Signer};

use sunshine_core::msg::NodeId;

use crate::{commands::solana::instructions::execute, CommandResult, Error, NftCreator, Value};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignMetadata {
    pub fee_payer: Option<NodeId>, // keypair
    pub mint_account: Option<NodeId>,
    pub creator: Option<NodeId>,
}

impl SignMetadata {
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

        let mint_account = match self.mint_account {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("mint_account") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("mint_account".to_string())),
            },
        };

        let creator = match self.creator {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("creator") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("creator".to_string())),
            },
        };

        let (metadata_account, _) = mpl_token_metadata::pda::find_metadata_account(&mint_account);

        let (minimum_balance_for_rent_exemption, instructions) =
            command_sign_metadata(metadata_account, creator.pubkey())?;

        let fee_payer_pubkey = fee_payer.pubkey();

        let signers: Vec<&dyn Signer> = vec![&creator, &fee_payer];

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
            "creator".to_owned() => Value::Keypair(creator.into()),
        };

        Ok(outputs)
    }
}

pub fn command_sign_metadata(metadata: Pubkey, creator: Pubkey) -> CommandResult {
    let instructions = vec![mpl_token_metadata::instruction::sign_metadata(
        mpl_token_metadata::id(),
        metadata,
        creator,
    )];

    Ok((0, instructions))
}
