use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey, signer::Signer, system_instruction};
use spl_token::state::Mint;

use crate::{error::Error, CommandResult, OutputType};

use super::{instructions::execute, Ctx};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateToken {
    pub fee_payer: Option<String>,
    pub decimals: Option<u8>,
    pub authority: Option<String>,
    pub token: Option<String>,
    pub memo: Option<String>,
}

impl CreateToken {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Mutex<Ctx>>,
        mut inputs: HashMap<String, OutputType>,
    ) -> Result<HashMap<String, OutputType>, Error> {
        let fee_payer = match &self.fee_payer {
            Some(s) => s.clone(),
            None => match inputs.remove("fee_payer") {
                Some(OutputType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("fee_payer".to_string())),
            },
        };

        let decimals = match &self.decimals {
            Some(s) => s.clone(),
            None => match inputs.remove("decimals") {
                Some(OutputType::U8(s)) => s,
                _ => return Err(Error::ArgumentNotFound("decimals".to_string())),
            },
        };

        let authority = match &self.authority {
            Some(s) => s.clone(),
            None => match inputs.remove("authority") {
                Some(OutputType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("authority".to_string())),
            },
        };

        let token = match &self.token {
            Some(s) => s.clone(),
            None => match inputs.remove("token") {
                Some(OutputType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("token".to_string())),
            },
        };

        let memo = match &self.memo {
            Some(s) => s.clone(),
            None => match inputs.remove("memo") {
                Some(OutputType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("memo".to_string())),
            },
        };

        let fee_payer = ctx.lock().unwrap().get_keypair(&fee_payer)?;
        let authority = ctx.lock().unwrap().get_keypair(&authority)?;
        let token = ctx.lock().unwrap().get_keypair(&token)?;

        let (minimum_balance_for_rent_exemption, instructions) = command_create_token(
            &ctx.lock().unwrap().client,
            &fee_payer.pubkey(),
            decimals,
            &token.pubkey(),
            authority.pubkey(),
            &memo,
        )?;

        let signers: Vec<Arc<dyn Signer>> =
            vec![authority.clone(), fee_payer.clone(), token.clone()];

        let signature = execute(
            &signers,
            &ctx.lock().unwrap().client,
            &fee_payer.pubkey(),
            &instructions,
            minimum_balance_for_rent_exemption,
        )?;

        Ok(hashmap! {
            "signature".to_owned()=>OutputType::Success(signature),
        })
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
