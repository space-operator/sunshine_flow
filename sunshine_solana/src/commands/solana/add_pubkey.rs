use std::{collections::HashMap, sync::Arc};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::convert::TryInto;

use crate::{error::Error, Value};

use super::Ctx;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddPubkey {
    pub name: Option<String>,
    pub pubkey: Option<Pubkey>,
}

impl AddPubkey {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let name = match &self.name {
            Some(s) => s.clone(),
            None => match inputs.remove("name") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("name".to_string())),
            },
        };

        let pubkey = match &self.pubkey {
            Some(p) => *p,
            None => match inputs.remove("pubkey") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("pubkey".to_string())),
            },
        };

        ctx.insert_pubkey(name, pubkey).await?;

        Ok(hashmap! {
            "pubkey".to_owned()=> Value::Pubkey(pubkey.into()),
        })
    }
}
