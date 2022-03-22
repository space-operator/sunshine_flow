use std::{collections::HashMap, str::FromStr, sync::Arc};

use either::Either;
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use sunshine_core::msg::NodeId;

use crate::{error::Error, Value};

use super::Ctx;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeletePubkey {
    pub input: Either<Option<String>, Option<NodeId>>,
}

impl DeletePubkey {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        match &self.input {
            Either::Left(pubkey) => {
                let pubkey = match pubkey {
                    Some(s) => s.clone(),
                    None => match inputs.remove("pubkey") {
                        Some(Value::String(s)) => s,
                        _ => return Err(Error::ArgumentNotFound("pubkey".to_string())),
                    },
                };
                let pubkey = Pubkey::from_str(&pubkey).unwrap();

                let node_id = ctx.get_node_id_by_pubkey(pubkey).await?;
                let pubkey = ctx.remove_pubkey(node_id).await?;

                Ok(hashmap! {
                    "removed_pubkey".to_owned()=> Value::Pubkey(pubkey.into())
                })
            }
            Either::Right(node_id) => {
                let node_id = match node_id {
                    Some(id) => *id,
                    None => match inputs.remove("node_id") {
                        Some(Value::NodeId(id)) => id,
                        _ => return Err(Error::ArgumentNotFound("node_id".to_string())),
                    },
                };
                let pubkey = ctx.remove_pubkey(node_id).await?;

                Ok(hashmap! {
                    "removed_pubkey".to_owned()=> Value::Pubkey(pubkey.into())
                })
            }
        }
    }
}
