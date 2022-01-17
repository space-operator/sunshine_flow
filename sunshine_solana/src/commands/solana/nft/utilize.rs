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
pub struct Utilize {
    pub token_account: Option<NodeId>,
    pub token: Option<NodeId>,
    pub use_authority_record_pda: Option<Option<NodeId>>,
    pub use_authority: Option<NodeId>, // keypair
    pub fee_payer: Option<NodeId>,     // keypair
    pub owner: Option<NodeId>,
    pub burner: Option<Option<NodeId>>,
    pub number_of_uses: Option<u64>,
}

impl Utilize {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
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

        let use_authority_record_pda = match self.use_authority_record_pda {
            Some(s) => match s {
                Some(use_authority_record_pda) => {
                    Some(ctx.get_pubkey_by_id(use_authority_record_pda).await?)
                }
                None => None,
            },
            None => match inputs.remove("use_authority_record_pda") {
                Some(Value::NodeIdOpt(s)) => match s {
                    Some(use_authority_record_pda) => {
                        Some(ctx.get_pubkey_by_id(use_authority_record_pda).await?)
                    }
                    None => None,
                },
                Some(Value::Keypair(k)) => {
                    let keypair: Keypair = k.into();
                    Some(keypair.pubkey())
                }
                Some(Value::Pubkey(k)) => Some(k),
                Some(Value::Empty) => None,
                _ => None,
            },
        };

        let use_authority = match self.use_authority {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("use_authority") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("use_authority".to_string())),
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

        let owner = match self.owner {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("owner") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("owner".to_string())),
            },
        };

        let burner = match self.burner {
            Some(s) => match s {
                Some(burner) => Some(ctx.get_pubkey_by_id(burner).await?),
                None => None,
            },
            None => match inputs.remove("burner") {
                Some(Value::NodeIdOpt(s)) => match s {
                    Some(burner) => Some(ctx.get_pubkey_by_id(burner).await?),
                    None => None,
                },
                Some(Value::Keypair(k)) => {
                    let keypair: Keypair = k.into();
                    Some(keypair.pubkey())
                }
                Some(Value::Pubkey(k)) => Some(k),
                Some(Value::Empty) => None,
                _ => None,
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

        let (minimum_balance_for_rent_exemption, instructions) = command_utilize(
            &ctx.client,
            metadata_pubkey,
            token_account,
            token,
            use_authority_record_pda,
            use_authority.pubkey(),
            fee_payer.pubkey(),
            owner,
            burner,
            number_of_uses,
        )?;

        let fee_payer_pubkey = fee_payer.pubkey();

        let signers: Vec<&dyn Signer> = vec![&use_authority, &fee_payer];

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

pub fn command_utilize(
    rpc_client: &RpcClient,
    metadata_pubkey: Pubkey,
    token_account: Pubkey,
    mint: Pubkey,
    use_authority_record_pda: Option<Pubkey>,
    use_authority: Pubkey,
    payer: Pubkey,
    owner: Pubkey,
    burner: Option<Pubkey>,
    number_of_uses: u64,
) -> CommandResult {
    let instructions = vec![mpl_token_metadata::instruction::utilize(
        mpl_token_metadata::id(),
        metadata_pubkey,
        token_account,
        mint,
        use_authority_record_pda,
        use_authority,
        payer,
        owner,
        burner,
        number_of_uses,
    )];

    Ok((0, instructions))
}
