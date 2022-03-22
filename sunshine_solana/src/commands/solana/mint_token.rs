use std::{collections::HashMap, str::FromStr, sync::Arc};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use spl_token::instruction::mint_to_checked;
use sunshine_core::msg::NodeId;

use crate::{error::Error, CommandResult, Value};

use super::{instructions::execute, Ctx};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MintToken {
    pub token: Option<NodeId>,
    pub recipient: Option<NodeId>,
    pub mint_authority: Option<NodeId>,
    pub amount: Option<f64>,
    pub fee_payer: Option<NodeId>,
}

impl MintToken {
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

        let recipient = match self.recipient {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("recipient") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("recipient".to_string())),
            },
        };

        let mint_authority = match self.mint_authority {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("mint_authority") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("mint_authority".to_string())),
            },
        };

        let amount = match self.amount {
            Some(s) => s,
            None => match inputs.remove("amount") {
                Some(Value::F64(s)) => s,
                _ => return Err(Error::ArgumentNotFound("amount".to_string())),
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

        let fee_payer_pubkey = fee_payer.pubkey();

        let (minimum_balance_for_rent_exemption, instructions) = command_mint(
            &ctx.client,
            token,
            fee_payer_pubkey,
            amount,
            recipient,
            mint_authority.pubkey(),
        )?;

        let signers: Vec<&dyn Signer> = vec![&mint_authority, &fee_payer];

        let signature = execute(
            &signers,
            &ctx.client,
            &fee_payer_pubkey,
            &instructions,
            minimum_balance_for_rent_exemption,
        )?;

        let outputs = hashmap! {
            "signature".to_owned() => Value::Success(signature),
            "token".to_owned()=> Value::Pubkey(token.into()),
            "fee_payer".to_owned() => Value::Keypair(fee_payer.into()),
            "recipient".to_owned() => Value::Pubkey(recipient.into()),
        };

        Ok(outputs)
    }
}

// checks mint account's decimals
// https://github.com/solana-labs/solana-program-library/blob/707382ee96c1197b50ab3e837b3c46b975e75a4f/token/cli/src/main.rs#L516
pub(crate) fn resolve_mint_info(
    client: &RpcClient,
    token_account: &Pubkey,
) -> Result<(Pubkey, u8), Error> {
    let source_account = client
        .get_token_account(token_account)
        .map_err(|_| Error::NotTokenAccount(token_account.to_string()))?
        .ok_or_else(|| Error::NotTokenAccount(token_account.to_string()))?;
    let source_mint = Pubkey::from_str(&source_account.mint).unwrap();
    Ok((source_mint, source_account.token_amount.decimals))
}

pub fn command_mint(
    client: &RpcClient,
    token: Pubkey,
    fee_payer: Pubkey,
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
        &[&fee_payer, &mint_authority],
        amount,
        decimals,
    )
    .unwrap()];

    Ok((0, instructions))
}
