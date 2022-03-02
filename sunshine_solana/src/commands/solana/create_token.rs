use std::{collections::HashMap, sync::Arc};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey, signer::Signer, system_instruction};
use spl_token::state::Mint;
use sunshine_core::msg::NodeId;

use crate::{error::Error, CommandResult, Value};

use super::{instructions::execute, Ctx};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateToken {
    pub fee_payer: Option<NodeId>,
    pub decimals: Option<u8>,
    pub authority: Option<NodeId>,
    pub token: Option<NodeId>,
    pub memo: Option<String>,
}

impl CreateToken {
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

        let decimals = match self.decimals {
            Some(s) => s,
            None => match inputs.remove("decimals") {
                Some(Value::U8(s)) => s,
                _ => return Err(Error::ArgumentNotFound("decimals".to_string())),
            },
        };

        let authority = match self.authority {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("authority") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("authority".to_string())),
            },
        };

        let token = match self.token {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("token") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("token".to_string())),
            },
        };

        let memo = match &self.memo {
            Some(s) => s.clone(),
            None => match inputs.remove("memo") {
                Some(Value::String(s)) => s,
                Some(Value::Empty) => String::new(),
                None => String::new(),
                _ => return Err(Error::ArgumentNotFound("memo".to_string())),
            },
        };

        let (minimum_balance_for_rent_exemption, instructions) = command_create_token(
            &ctx.client,
            &fee_payer.pubkey(),
            decimals,
            &token.pubkey(),
            authority.pubkey(),
            &memo,
        )?;

        let fee_payer_pubkey = fee_payer.pubkey();

        let signers: Vec<&dyn Signer> = vec![&authority, &fee_payer, &token];

        let res = execute(
            &signers,
            &ctx.client,
            &fee_payer_pubkey,
            &instructions,
            minimum_balance_for_rent_exemption,
        );

        let signature = res?;

        let outputs = hashmap! {
            "token".to_owned()=> Value::Keypair(token.into()),
            "signature".to_owned()=>Value::Success(signature),
            "fee_payer".to_owned() => Value::Keypair(fee_payer.into()),
            "authority".to_owned() => Value::Keypair(authority.into()),
        };

        Ok(outputs)
    }
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
