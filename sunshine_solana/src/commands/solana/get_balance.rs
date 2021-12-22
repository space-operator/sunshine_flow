use std::{collections::HashMap, sync::Arc};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use sunshine_core::msg::NodeId;

use crate::{error::Error, Value};

use super::Ctx;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetBalance {
    pub pubkey: Option<NodeId>,
}

impl GetBalance {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let pubkey = match self.pubkey {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("pubkey") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("pubkey".to_string())),
            },
        };

        let balance = ctx.client.get_balance(&pubkey)?;

        Ok(hashmap! {
            "balance".to_owned()=> Value::Balance(balance),
        })
    }
}
