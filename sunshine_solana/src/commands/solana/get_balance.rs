use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use sunshine_core::msg::NodeId;

use crate::{error::Error, ValueType};

use super::Ctx;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetBalance {
    pub node_id: Option<NodeId>,
}

impl GetBalance {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, ValueType>,
    ) -> Result<HashMap<String, ValueType>, Error> {
        let node_id = match &self.node_id {
            Some(node_id) => *node_id,
            None => match inputs.remove("node_id") {
                Some(ValueType::NodeId(node_id)) => node_id,
                _ => return Err(Error::ArgumentNotFound("node_id".to_string())),
            },
        };

        let pubkey = ctx.get_pubkey(node_id).await?;

        let balance = ctx.client.get_balance(&pubkey)?;

        Ok(hashmap! {
            "balance".to_owned()=> ValueType::Balance(balance),
        })
    }
}
