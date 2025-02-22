use std::{collections::HashMap, sync::Arc};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    program_pack::Pack, pubkey::Pubkey, signer::Signer, system_instruction, system_program,
};
use sunshine_core::msg::NodeId;

use crate::CommandResult;

use crate::{error::Error, Value};

use super::{instructions::execute, Ctx};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateTokenAccount {
    pub owner: Option<NodeId>,
    pub fee_payer: Option<NodeId>,
    pub mint_account: Option<NodeId>,
    pub token_account: Option<Option<NodeId>>,
}

impl CreateTokenAccount {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let owner = match self.owner {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("owner") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
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

        let mint_account = match self.mint_account {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("mint_account") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("mint_account".to_string())),
            },
        };

        let token_account = match self.token_account {
            Some(s) => match s {
                Some(token_account) => Some(ctx.get_keypair_by_id(token_account).await?),
                None => None,
            },
            None => match inputs.remove("token_account") {
                Some(Value::NodeIdOpt(s)) => match s {
                    Some(token_account) => Some(ctx.get_keypair_by_id(token_account).await?),
                    None => None,
                },
                Some(Value::Keypair(k)) => Some(k.into()),
                Some(Value::Empty) => None,
                _ => None,
            },
        };

        let (minimum_balance_for_rent_exemption, instructions) = command_create_token_account(
            &ctx.client,
            fee_payer.pubkey(),
            mint_account,
            owner,
            token_account.as_ref().map(|acc| acc.pubkey()),
        )
        .unwrap();

        let fee_payer_pubkey = fee_payer.pubkey();

        let mut signers: Vec<&dyn Signer> = vec![&fee_payer];

        if let Some(token_account) = token_account.as_ref() {
            signers.push(token_account);
        };

        let signature = execute(
            &signers,
            &ctx.client,
            &fee_payer_pubkey,
            &instructions,
            minimum_balance_for_rent_exemption,
        )?;

        let mut outputs = hashmap! {
            "signature".to_owned() => Value::Success(signature),
            "mint_account".to_owned()=> Value::Pubkey(mint_account.into()),
            "owner".to_owned() => Value::Pubkey(owner.into()),
            "fee_payer".to_owned() => Value::Keypair(fee_payer.into()),
        };

        if let Some(token_account) = token_account {
            outputs.insert(
                "token_account".into(),
                Value::Pubkey(token_account.pubkey().into()),
            );
        } else {
            outputs.insert(
                "token_account".into(),
                Value::Pubkey(
                    spl_associated_token_account::get_associated_token_address(
                        &owner,
                        &mint_account,
                    )
                    .into(),
                ),
            );
        }

        Ok(outputs)
    }
}

pub fn command_create_token_account(
    client: &RpcClient,
    fee_payer: Pubkey,
    token: Pubkey,
    owner: Pubkey,
    maybe_account: Option<Pubkey>,
) -> CommandResult {
    let minimum_balance_for_rent_exemption = client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)
        .unwrap();

    let (account, system_account_ok, instructions) = if let Some(account) = maybe_account {
        (
            account,
            false,
            vec![
                system_instruction::create_account(
                    &fee_payer,
                    &account,
                    minimum_balance_for_rent_exemption,
                    spl_token::state::Account::LEN as u64,
                    &spl_token::id(),
                ),
                spl_token::instruction::initialize_account(
                    &spl_token::id(),
                    &account,
                    &token,
                    &owner,
                )
                .unwrap(),
            ],
        )
    } else {
        let account = spl_associated_token_account::get_associated_token_address(&owner, &token);
        (
            account,
            true,
            vec![
                spl_associated_token_account::create_associated_token_account(
                    &fee_payer, &owner, &token,
                ),
            ],
        )
    };

    if let Some(account_data) = client
        .get_account_with_commitment(&account, client.commitment())
        .unwrap()
        .value
    {
        if !(account_data.owner == system_program::id() && system_account_ok) {
            panic!("Error: Account already exists: {}", account);
        }
    }

    Ok((minimum_balance_for_rent_exemption, instructions))
}

//
