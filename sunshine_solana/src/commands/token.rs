use std::str::FromStr;

use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use spl_token::{instruction::mint_to_checked, state::Mint};

use crate::CommandResult;

type Error = Box<dyn std::error::Error>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateToken {
    pub fee_payer: String,
    pub decimals: u8,
    pub authority: String,
    pub token: String,
    pub memo: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MintToken {
    pub token: String,
    pub recipient: String,
    pub mint_authority: String,
    pub amount: f64,
    pub fee_payer: String,
}

pub fn command_create_token(
    rpc_client: &RpcClient,
    fee_payer: &Pubkey,
    decimals: u8,
    token: &Pubkey,
    authority: Pubkey,
    memo: &str,
) -> CommandResult {
    let minimum_balance_for_rent_exemption =
        rpc_client.get_minimum_balance_for_rent_exemption(Mint::LEN)?;

    let freeze_authority_pubkey = Some(authority);

    let instructions = vec![
        system_instruction::create_account(
            fee_payer,
            token,
            minimum_balance_for_rent_exemption,
            Mint::LEN as u64,
            &spl_token::id(),
        ),
        spl_token::instruction::initialize_mint(
            &spl_token::id(),
            token,
            &authority,
            freeze_authority_pubkey.as_ref(),
            decimals,
        )?,
        spl_memo::build_memo(memo.as_bytes(), &[fee_payer]),
    ];

    Ok((minimum_balance_for_rent_exemption, instructions))
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
