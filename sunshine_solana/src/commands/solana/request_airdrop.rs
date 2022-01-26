use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use sunshine_core::msg::NodeId;

use crate::{error::Error, Value};

use super::Ctx;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RequestAirdrop {
    pub pubkey: Option<NodeId>,
    pub amount: Option<u64>,
}

impl RequestAirdrop {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let pubkey = match &self.pubkey {
            Some(p) => ctx.get_pubkey_by_id(*p).await?,
            None => match inputs.remove("pubkey") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("pubkey".to_string())),
            },
        };

        let amount = match &self.amount {
            Some(s) => *s,
            None => match inputs.remove("amount") {
                Some(Value::U64(s)) => s,
                _ => return Err(Error::ArgumentNotFound("amount".to_string())),
            },
        };

        let signature = ctx.client.request_airdrop(&pubkey, amount)?;

        tokio::time::sleep(Duration::from_secs(30)).await;

        let succeeded = ctx.client.confirm_transaction(&signature)?;

        if !succeeded {
            return Err(Error::AirdropFailed);
        }

        Ok(hashmap! {
            "signature".to_owned()=> Value::Success(signature),
        })
    }
}
