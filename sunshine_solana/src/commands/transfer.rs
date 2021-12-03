use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey, system_program};
use spl_token::instruction::transfer_checked;

use crate::CommandResult;

use super::token::resolve_mint_info;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transfer {
    pub fee_payer: String,
    pub token: String,
    pub amount: f64,
    pub recipient: String,
    pub sender: Option<String>,
    pub sender_owner: String,
    pub allow_unfunded_recipient: bool,
    pub fund_recipient: bool,
    pub memo: Option<String>,
}

// https://spl.solana.com/associated-token-account
// https://github.com/solana-labs/solana-program-library/blob/master/token/cli/src/main.rs#L555
#[allow(clippy::too_many_arguments)]
pub fn command_transfer(
    client: &RpcClient,
    fee_payer: &Pubkey,
    token: Pubkey,
    ui_amount: f64,
    recipient: Pubkey,
    sender: Option<Pubkey>,
    sender_owner: Pubkey,
    allow_unfunded_recipient: bool,
    fund_recipient: bool,
    memo: Option<String>,
) -> CommandResult {
    let sender = if let Some(sender) = sender {
        sender
    } else {
        spl_associated_token_account::get_associated_token_address(&sender_owner, &token)
    };
    let (_, decimals) = resolve_mint_info(client, &recipient).unwrap();
    let transfer_balance = spl_token::ui_amount_to_amount(ui_amount, decimals);
    let transfer_balance = {
        let sender_token_amount = client
            .get_token_account_balance(&sender)
            .map_err(|err| {
                format!(
                    "Error: Failed to get token balance of sender address {}: {}",
                    sender, err
                )
            })
            .unwrap();

        let sender_balance = sender_token_amount
            .amount
            .parse::<u64>()
            .map_err(|err| {
                format!(
                    "Token account {} balance could not be parsed: {}",
                    sender, err
                )
            })
            .unwrap();

        if transfer_balance > sender_balance {
            panic!(
                "Error: Sender has insufficient funds, current balance is {}",
                sender_token_amount.real_number_string_trimmed()
            );
        }

        transfer_balance
    };

    let mut instructions = vec![];

    let mut recipient_token_account = recipient;
    let mut minimum_balance_for_rent_exemption = 0;

    let recipient_is_token_account = {
        let recipient_account_info = client
            .get_account_with_commitment(&recipient, client.commitment())?
            .value
            .map(|account| {
                account.owner == spl_token::id()
                    && account.data.len() == spl_token::state::Account::LEN
            });

        if recipient_account_info.is_none() && !allow_unfunded_recipient {
            return Err("Error: The recipient address is not funded. \
                                    Add `--allow-unfunded-recipient` to complete the transfer \
                                   "
            .into());
        }

        recipient_account_info.unwrap_or(false)
    };

    if !recipient_is_token_account {
        recipient_token_account =
            spl_associated_token_account::get_associated_token_address(&recipient, &token);

        let needs_funding = {
            if let Some(recipient_token_account_data) = client
                .get_account_with_commitment(&recipient_token_account, client.commitment())?
                .value
            {
                if recipient_token_account_data.owner == system_program::id() {
                    true
                } else if recipient_token_account_data.owner == spl_token::id() {
                    false
                } else {
                    return Err(
                        format!("Error: Unsupported recipient address: {}", recipient).into(),
                    );
                }
            } else {
                true
            }
        };

        if needs_funding {
            if fund_recipient {
                minimum_balance_for_rent_exemption += client
                    .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)?;
                instructions.push(
                    spl_associated_token_account::create_associated_token_account(
                        fee_payer, &recipient, &token,
                    ),
                );
            } else {
                return Err(
                    "Error: Recipient's associated token account does not exist. \
                                    Add `--fund-recipient` to fund their account"
                        .into(),
                );
            }
        }
    }

    instructions.push(
        transfer_checked(
            &spl_token::id(),
            &sender,
            &token,
            &recipient_token_account,
            &sender_owner,
            &[&sender, fee_payer],
            transfer_balance,
            decimals,
        )
        .unwrap(),
    );

    if let Some(text) = memo {
        instructions.push(spl_memo::build_memo(text.as_bytes(), &[fee_payer]));
    }

    Ok((minimum_balance_for_rent_exemption, instructions))
}
