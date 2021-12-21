use std::{collections::HashMap, sync::Arc};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
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
            Some(s) => *s,
            None => match inputs.remove("pubkey") {
                Some(Value::NodeId(s)) => s,
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

        let pubkey = ctx.get_pubkey_by_id(pubkey).await?;

        let signature = ctx.client.request_airdrop(&pubkey, amount)?;

        Ok(hashmap! {
            "signature".to_owned()=> Value::Success(signature),
        })
    }
}
