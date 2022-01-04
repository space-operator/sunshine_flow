use std::{collections::HashMap, sync::Arc};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;

use crate::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InitializeCandyMachine {
    price: Option<u64>,
    max_supply: Option<u64>,
    wallet: Option<NodeId>,
    token_mind: Option<Option<NodeId>>,
    go_live_date: Option<Option<i64>>,
}

impl InitializeCandyMachine {
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
