use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey, system_instruction, system_program};

use crate::CommandResult;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateAccount {
    pub owner: String,
    pub fee_payer: String,
    pub token: String,
    pub account: Option<String>,
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
