use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::Keypair;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey, signer::Signer, system_program};
use spl_token::instruction::transfer_checked;
use std::{collections::HashMap, sync::Arc};
use sunshine_core::msg::NodeId;

use crate::{error::Error, Value};

use super::instructions::execute;
use super::mint_token::resolve_mint_info;
use super::Ctx;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransferToken {
    pub fee_payer: Option<NodeId>,
    pub token: Option<NodeId>,
    pub amount: Option<f64>,
    pub recipient: Option<NodeId>,
    pub sender: Option<Option<NodeId>>,
    pub sender_owner: Option<NodeId>,
    pub allow_unfunded: Option<bool>,
    pub fund_recipient: Option<bool>,
    pub memo: Option<Option<String>>,
}

impl TransferToken {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let fee_payer = match self.fee_payer {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("fee_payer") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("fee_payer".to_string())),
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

        let amount = match self.amount {
            Some(s) => s,
            None => match inputs.remove("amount") {
                Some(Value::F64(s)) => s,
                _ => return Err(Error::ArgumentNotFound("amount".to_string())),
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

        let sender = match self.sender {
            //TODOrename sender_account
            Some(s) => match s {
                Some(sender) => Some(ctx.get_pubkey_by_id(sender).await?),
                None => None,
            },
            None => match inputs.remove("sender") {
                Some(Value::NodeIdOpt(s)) => match s {
                    Some(sender) => Some(ctx.get_pubkey_by_id(sender).await?),
                    None => None,
                },
                Some(Value::Keypair(k)) => Some(Keypair::from(k).pubkey()),
                Some(Value::Pubkey(p)) => Some(p.into()),
                Some(Value::Empty) => None,
                None => None,
                _ => return Err(Error::ArgumentNotFound("sender".to_string())),
            },
        };

        let sender_owner = match self.sender_owner {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("sender_owner") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("sender_owner".to_string())),
            },
        };

        let allow_unfunded = match self.allow_unfunded {
            Some(s) => s,
            None => match inputs.remove("allow_unfunded") {
                Some(Value::Bool(s)) => s,
                None => true,
                _ => return Err(Error::ArgumentNotFound("allow_unfunded".to_string())),
            },
        };

        let fund_recipient = match self.fund_recipient {
            Some(s) => s,
            None => match inputs.remove("fund_recipient") {
                Some(Value::Bool(s)) => s,
                None => true,
                _ => return Err(Error::ArgumentNotFound("fund_recipient".to_string())),
            },
        };

        let memo: Option<String> = match self.memo.clone() {
            Some(val) => val,
            None => match inputs.remove("memo") {
                Some(Value::StringOpt(s)) => s,
                Some(Value::String(s)) => Some(s),
                Some(Value::Empty) => None,
                None => None,
                _ => return Err(Error::ArgumentNotFound("memo".to_string())),
            },
        };

        let (minimum_balance_for_rent_exemption, instructions, recipient_acc) = command_transfer_token(
            &ctx.client,
            &fee_payer.pubkey(),
            token,
            amount,
            recipient,
            sender,
            sender_owner.pubkey(),
            allow_unfunded,
            fund_recipient,
            memo,
        )?;

        let fee_payer_pubkey = fee_payer.pubkey();

        let signers: Vec<&dyn Signer> = vec![&fee_payer, &sender_owner];

        let signature = execute(
            &signers,
            &ctx.client,
            &fee_payer_pubkey,
            &instructions,
            minimum_balance_for_rent_exemption,
        )?;

        let outputs = hashmap! {
            "sender_owner".to_owned()=> Value::Pubkey(sender_owner.pubkey().into()),
            "recipient_account".to_owned()=> Value::Pubkey(recipient_acc.into()),
            "signature".to_owned() => Value::Success(signature),
            "fee_payer".to_owned() => Value::Keypair(fee_payer.into()),
        };

        Ok(outputs)
    }
}

// https://spl.solana.com/associated-token-account
// https://github.com/solana-labs/solana-program-library/blob/master/token/cli/src/main.rs#L555
#[allow(clippy::too_many_arguments)]
pub fn command_transfer_token(
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
) -> Result<(u64, Vec<Instruction>, Pubkey), Error> {
    let sender = if let Some(sender) = sender {
        sender
    } else {
        spl_associated_token_account::get_associated_token_address(&sender_owner, &token)
    };
    let (_, decimals) = resolve_mint_info(client, &sender)?;
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
            return Err(Error::RecipientAddressNotFunded);
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
                    return Err(Error::UnsupportedRecipientAddress(recipient.to_string()));
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
                return Err(Error::AssociatedTokenAccountDoesntExist);
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
            &[&sender_owner, fee_payer],
            transfer_balance,
            decimals,
        )
        .unwrap(),
    );

    if let Some(text) = memo {
        instructions.push(spl_memo::build_memo(text.as_bytes(), &[fee_payer]));
    }

    Ok((
        minimum_balance_for_rent_exemption,
        instructions,
        recipient_token_account,
    ))
}
