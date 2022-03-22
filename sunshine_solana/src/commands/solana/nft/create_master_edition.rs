use std::{collections::HashMap, sync::Arc};

use super::super::Ctx;
use maplit::hashmap;
use mpl_token_metadata::state::Creator;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

use sunshine_core::msg::NodeId;

use crate::{commands::solana::instructions::execute, CommandResult, Error, NftCreator, Value};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateMasterEdition {
    pub token: Option<NodeId>,
    pub token_authority: Option<NodeId>,
    pub fee_payer: Option<NodeId>,        // keypair
    pub update_authority: Option<NodeId>, // keypair
    pub max_supply: Arg,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Arg {
    Some(Option<u64>),
    None,
}

impl CreateMasterEdition {
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

        let max_supply: Option<u64> = match self.max_supply.clone() {
            Arg::Some(val) => val,
            Arg::None => match inputs.remove("max_supply") {
                Some(Value::U64(s)) => Some(s),
                Some(Value::Empty) => None,
                None => None,
                _ => return Err(Error::ArgumentNotFound("max_supply".to_string())),
            },
        };

        let program_id = mpl_token_metadata::id();

        let metadata_seeds = &[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            &program_id.as_ref(),
            token.as_ref(),
        ];

        let (metadata_pubkey, _) = Pubkey::find_program_address(metadata_seeds, &program_id);

        let master_edition_seeds = &[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            &program_id.as_ref(),
            token.as_ref(),
            "edition".as_bytes(),
        ];

        let (master_edition_pubkey, _) =
            Pubkey::find_program_address(master_edition_seeds, &program_id);

        let (minimum_balance_for_rent_exemption, instructions) = command_create_master_edition(
            &ctx.client,
            metadata_pubkey,
            master_edition_pubkey,
            token,
            token_authority,
            fee_payer.pubkey(),
            update_authority.pubkey(),
            max_supply,
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
            "token".to_owned()=>Value::Pubkey(token.into()),
            "metadata_pubkey".to_owned()=>Value::Pubkey(metadata_pubkey.into()),
            "master_edition_pubkey".to_owned()=>Value::Pubkey(master_edition_pubkey.into()),
        };

        Ok(outputs)
    }
}

pub fn command_create_master_edition(
    rpc_client: &RpcClient,
    metadata_pubkey: Pubkey,
    master_edition_pubkey: Pubkey,
    mint: Pubkey,
    mint_authority: Pubkey,
    payer: Pubkey,
    update_authority: Pubkey,
    max_supply: Option<u64>,
) -> CommandResult {
    let minimum_balance_for_rent_exemption =
        rpc_client.get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            mpl_token_metadata::state::MasterEditionV2,
        >())?;

    let instructions = vec![mpl_token_metadata::instruction::create_master_edition_v3(
        mpl_token_metadata::id(),
        master_edition_pubkey,
        mint,
        update_authority,
        mint_authority,
        metadata_pubkey,
        payer,
        max_supply,
    )];

    Ok((minimum_balance_for_rent_exemption, instructions))
}
