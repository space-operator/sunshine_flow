use std::{collections::HashMap, sync::Arc};

use either::Either;
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use sunshine_core::msg::NodeId;

use crate::{error::Error, Value};

use super::Ctx;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeleteKeypair {
    pub input: Either<Option<String>, Option<NodeId>>,
}

impl DeleteKeypair {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        match &self.input {
            Either::Left(keypair) => {
                let keypair = match keypair {
                    Some(s) => s.clone(),
                    None => match inputs.remove("keypair") {
                        Some(Value::String(s)) => s,
                        _ => return Err(Error::ArgumentNotFound("keypair".to_string())),
                    },
                };

                let node_id = ctx.get_node_id_by_keypair(keypair.as_str()).await?;
                let keypair = ctx.remove_keypair(node_id).await?;

                Ok(hashmap! {
                    "removed_keypair".to_owned()=> Value::Keypair(keypair.into())
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
                let keypair = ctx.remove_keypair(node_id).await?;

                Ok(hashmap! {
                    "removed_keypair".to_owned()=> Value::Keypair(keypair.into())
                })
            }
        }
    }
}
