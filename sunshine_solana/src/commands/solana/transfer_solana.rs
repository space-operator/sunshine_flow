use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_sdk::signer::Signer;
use std::{collections::HashMap, sync::Arc};
use sunshine_core::msg::NodeId;

use crate::{error::Error, Value};

use super::Ctx;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransferSolana {
    pub sender: Option<NodeId>,
    pub recipient: Option<NodeId>,
    pub amount: Option<f64>,
}

impl TransferSolana {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let sender = match self.sender {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("sender") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("sender".to_string())),
            },
        };

        let amount = match self.amount {
            Some(s) => s,
            None => match inputs.remove("amount") {
                Some(Value::F64(s)) => s,
                _ => return Err(Error::ArgumentNotFound("amount".to_string())),
            },
        };

        // TODO implement better than Solana
        let amount = solana_sdk::native_token::sol_to_lamports(amount);

        let recipient = match self.recipient {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("recipient") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("recipient".to_string())),
            },
        };

        let recent_blockhash = ctx.client.get_latest_blockhash()?;

        let tx =
            solana_sdk::system_transaction::transfer(&sender, &recipient, amount, recent_blockhash);

        let signature = ctx.client.send_and_confirm_transaction(&tx)?;

        let outputs = hashmap! {
            "sender".to_owned()=> Value::Keypair(sender.into()),
            "recipient".to_owned()=> Value::Pubkey(recipient),
            "signature".to_owned() => Value::Success(signature),
        };

        Ok(outputs)
    }
}
