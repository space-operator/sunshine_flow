use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    program_pack::Pack, pubkey::Pubkey, signer::Signer, system_instruction, system_program,
};

use crate::CommandResult;

use crate::{error::Error, ValueType};

use super::{instructions::execute, Ctx};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateAccount {
    pub owner: Option<String>,
    pub fee_payer: Option<String>,
    pub token: Option<String>,
    pub account: Option<String>,
}

impl CreateAccount {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Mutex<Ctx>>,
        mut inputs: HashMap<String, ValueType>,
    ) -> Result<HashMap<String, ValueType>, Error> {
        let owner = match &self.owner {
            Some(s) => s.clone(),
            None => match inputs.remove("owner") {
                Some(ValueType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("owner".to_string())),
            },
        };

        let fee_payer = match &self.fee_payer {
            Some(s) => s.clone(),
            None => match inputs.remove("fee_payer") {
                Some(ValueType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("fee_payer".to_string())),
            },
        };

        let token = match &self.token {
            Some(s) => s.clone(),
            None => match inputs.remove("token") {
                Some(ValueType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("token".to_string())),
            },
        };
        let account = match &self.account {
            Some(s) => s.clone(),
            None => match inputs.remove("account") {
                Some(ValueType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("account".to_string())),
            },
        };

        let owner = ctx.get_pubkey(&owner)?;
        let fee_payer = ctx.get_keypair(&fee_payer)?;
        let token = ctx.get_pubkey(&token)?;
        let account = match self.account {
            Some(ref account) => Some(ctx.get_keypair(account)?),
            None => None,
        };

        let (minimum_balance_for_rent_exemption, instructions) = command_create_account(
            &ctx.client,
            fee_payer.pubkey(),
            token,
            owner,
            account.as_ref().map(|a| a.pubkey()),
        )
        .unwrap();

        let mut signers: Vec<Arc<dyn Signer>> = vec![fee_payer.clone()];

        if let Some(account) = account {
            signers.push(account.clone());
        };

        let signature = execute(
            &signers,
            &ctx.client,
            &fee_payer.pubkey(),
            &instructions,
            minimum_balance_for_rent_exemption,
        )?;

        Ok(hashmap! {
             "signature".to_owned() => ValueType::Success(signature),
        })
    }
}

pub fn command_create_account(
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
