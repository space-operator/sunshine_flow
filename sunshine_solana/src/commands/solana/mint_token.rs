use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use spl_token::instruction::mint_to_checked;

use crate::{error::Error, CommandResult, ValueType};

use super::{instructions::execute, Ctx};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MintToken {
    pub token: Option<String>,
    pub recipient: Option<String>,
    pub mint_authority: Option<String>,
    pub amount: Option<f64>,
    pub fee_payer: Option<String>,
}

impl MintToken {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Mutex<Ctx>>,
        mut inputs: HashMap<String, ValueType>,
    ) -> Result<HashMap<String, ValueType>, Error> {
        let token = match &self.token {
            Some(s) => s.clone(),
            None => match inputs.remove("token") {
                Some(ValueType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("token".to_string())),
            },
        };

        let recipient = match &self.recipient {
            Some(s) => s.clone(),
            None => match inputs.remove("recipient") {
                Some(ValueType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("recipient".to_string())),
            },
        };
        let mint_authority = match &self.mint_authority {
            Some(s) => s.clone(),
            None => match inputs.remove("mint_authority") {
                Some(ValueType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("mint_authority".to_string())),
            },
        };
        let amount = match &self.amount {
            Some(s) => s.clone(),
            None => match inputs.remove("amount") {
                Some(ValueType::Float(s)) => s,
                _ => return Err(Error::ArgumentNotFound("amount".to_string())),
            },
        };
        let fee_payer = match &self.fee_payer {
            Some(s) => s.clone(),
            None => match inputs.remove("fee_payer") {
                Some(ValueType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("fee_payer".to_string())),
            },
        };

        let token = ctx.lock().unwrap().get_keypair(&token)?;
        let mint_authority = ctx.lock().unwrap().get_keypair(&mint_authority)?;
        let recipient = ctx.lock().unwrap().get_pubkey(&recipient)?;
        let fee_payer = ctx.lock().unwrap().get_keypair(&fee_payer)?;

        let (minimum_balance_for_rent_exemption, instructions) = command_mint(
            &ctx.lock().unwrap().client,
            token.pubkey(),
            amount,
            recipient,
            mint_authority.pubkey(),
        )?;

        let signers: Vec<Arc<dyn Signer>> =
            vec![mint_authority.clone(), token.clone(), fee_payer.clone()];

        let signature = execute(
            &signers,
            &ctx.lock().unwrap().client,
            &fee_payer.pubkey(),
            &instructions,
            minimum_balance_for_rent_exemption,
        )?;

        Ok(hashmap! {
            "signature".to_owned() => ValueType::Success(signature),
        })
    }
}

// checks mint account's decimals
// https://github.com/solana-labs/solana-program-library/blob/707382ee96c1197b50ab3e837b3c46b975e75a4f/token/cli/src/main.rs#L516
pub(crate) fn resolve_mint_info(
    client: &RpcClient,
    token_account: &Pubkey,
) -> Result<(Pubkey, u8), Error> {
    let source_account = client.get_token_account(token_account).unwrap().unwrap();
    let source_mint = Pubkey::from_str(&source_account.mint).unwrap();
    Ok((source_mint, source_account.token_amount.decimals))
}

pub fn command_mint(
    client: &RpcClient,
    token: Pubkey,
    ui_amount: f64,
    recipient: Pubkey,
    mint_authority: Pubkey,
) -> CommandResult {
    let (_, decimals) = resolve_mint_info(client, &recipient)?;
    let amount = spl_token::ui_amount_to_amount(ui_amount, decimals);

    let instructions = vec![mint_to_checked(
        &spl_token::id(),
        &token,
        &recipient,
        &mint_authority,
        &[&token, &mint_authority],
        amount,
        decimals,
    )
    .unwrap()];

    Ok((0, instructions))
}
