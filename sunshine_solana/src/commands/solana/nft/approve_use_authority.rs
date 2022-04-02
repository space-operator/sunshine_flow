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
    pub use_authority: Option<NodeId>,
    pub fee_payer: Option<NodeId>, // keypair
    pub token_account: Option<Option<NodeId>>,
    pub owner: Option<NodeId>,
    pub mint_account: Option<NodeId>,
    pub burner: Option<NodeId>,
    pub number_of_uses: Option<u64>,
}

impl ApproveUseAuthority {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let use_authority = match self.use_authority {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("use_authority") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("use_authority".to_string())),
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
            Some(s) => match s {
                Some(token_account) => Some(ctx.get_pubkey_by_id(token_account).await?),
                None => None,
            },
            None => match inputs.remove("token_account") {
                Some(Value::NodeIdOpt(s)) => match s {
                    Some(token_account) => Some(ctx.get_pubkey_by_id(token_account).await?),
                    None => None,
                },
                Some(Value::Keypair(k)) => Some(Keypair::from(k).pubkey()),
                Some(Value::Pubkey(p)) => Some(p.into()),
                Some(Value::Empty) => None,
                None => None,
                _ => return Err(Error::ArgumentNotFound("token_account".to_string())),
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
            mint_account.as_ref(),
        ];

        let (metadata_pubkey, _) = Pubkey::find_program_address(metadata_seeds, &program_id);

        let use_authority_seeds = &[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            &program_id.as_ref(),
            &mint_account.as_ref(),
            mpl_token_metadata::state::USER.as_bytes(),
            &use_authority.as_ref(),
        ];

        let (use_authority_record_pubkey, _) =
            Pubkey::find_program_address(use_authority_seeds, &program_id);

        let token_account = token_account.unwrap_or_else(|| {
            spl_associated_token_account::get_associated_token_address(
                &owner.pubkey(),
                &mint_account,
            )
        });

        let (minimum_balance_for_rent_exemption, instructions) = command_approve_use_authority(
            &ctx.client,
            use_authority_record_pubkey,
            use_authority,
            owner.pubkey(),
            fee_payer.pubkey(),
            token_account,
            metadata_pubkey,
            mint_account,
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
            "fee_payer".to_owned() => Value::Keypair(fee_payer.into()),
            "mint_account".to_owned()=> Value::Pubkey(mint_account.into()),
            "use_authority".to_owned() => Value::Pubkey(use_authority.into()),
            "owner".to_owned() => Value::Keypair(owner.into()),
            "token_account".to_owned() => Value::Pubkey(token_account.into()),
            "burner".to_owned() => Value::Pubkey(burner.into()),
            "use_authority_record".to_owned() => Value::Pubkey(use_authority_record_pubkey.into()),
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
