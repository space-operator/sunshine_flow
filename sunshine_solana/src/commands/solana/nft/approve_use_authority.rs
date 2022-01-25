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
pub struct ApproveUseAuthority {
    pub user: Option<NodeId>,
    pub owner: Option<NodeId>,
    pub fee_payer: Option<NodeId>, // keypair
    pub token_account: Option<NodeId>,
    pub token: Option<NodeId>,
    pub burner: Option<NodeId>,
    pub number_of_uses: Option<u64>,
}

impl ApproveUseAuthority {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let user = match self.user {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("user") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("user".to_string())),
            },
        };

        let owner = match self.owner {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("owner") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("owner".to_string())),
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

        let token_account = match self.token_account {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("token_account") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("token_account".to_string())),
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

        let burner = match self.burner {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("burner") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("burner".to_string())),
            },
        };

        let number_of_uses = match self.number_of_uses {
            Some(s) => s,
            None => match inputs.remove("number_of_uses") {
                Some(Value::U64(s)) => s,
                _ => return Err(Error::ArgumentNotFound("number_of_uses".to_string())),
            },
        };

        let program_id = mpl_token_metadata::id();

        let metadata_seeds = &[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            &program_id.as_ref(),
            token.as_ref(),
        ];

        let (metadata_pubkey, _) = Pubkey::find_program_address(metadata_seeds, &program_id);

        let use_authority_seeds = &[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            &program_id.as_ref(),
            &token.as_ref(),
            mpl_token_metadata::state::USER.as_bytes(),
            &user.as_ref(),
        ];

        let (use_authority_record_pubkey, _) =
            Pubkey::find_program_address(use_authority_seeds, &program_id);

        let (minimum_balance_for_rent_exemption, instructions) = command_approve_use_authority(
            &ctx.client,
            use_authority_record_pubkey,
            user,
            owner.pubkey(),
            fee_payer.pubkey(),
            token_account,
            metadata_pubkey,
            token,
            burner,
            number_of_uses,
        )?;

        let fee_payer_pubkey = fee_payer.pubkey();

        let signers: Vec<&dyn Signer> = vec![&owner, &fee_payer];

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
            "use_authority_record_pubkey".to_owned() => Value::Pubkey(use_authority_record_pubkey),
        };

        Ok(outputs)
    }
}

pub fn command_approve_use_authority(
    rpc_client: &RpcClient,
    use_authority_record_pubkey: Pubkey,
    user: Pubkey,
    owner: Pubkey,
    payer: Pubkey,
    token_account: Pubkey,
    metadata_pubkey: Pubkey,
    mint: Pubkey,
    burner: Pubkey,
    number_of_uses: u64,
) -> CommandResult {
    let minimum_balance_for_rent_exemption =
        rpc_client.get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            mpl_token_metadata::state::UseAuthorityRecord,
        >())?;

    let instructions = vec![mpl_token_metadata::instruction::approve_use_authority(
        mpl_token_metadata::id(),
        use_authority_record_pubkey,
        user,
        owner,
        payer,
        token_account,
        metadata_pubkey,
        mint,
        burner,
        number_of_uses,
    )];

    Ok((minimum_balance_for_rent_exemption, instructions))
}
