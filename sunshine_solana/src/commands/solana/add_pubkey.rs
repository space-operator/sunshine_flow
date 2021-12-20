use std::{collections::HashMap, sync::Arc};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

use crate::{error::Error, ValueType};

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
        mut inputs: HashMap<String, ValueType>,
    ) -> Result<HashMap<String, ValueType>, Error> {
        let name = match &self.name {
            Some(s) => s.clone(),
            None => match inputs.remove("name") {
                Some(ValueType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("name".to_string())),
            },
        };

        let pubkey = match &self.pubkey {
            Some(p) => *p,
            None => match inputs.remove("pubkey") {
                Some(ValueType::Pubkey(p)) => p,
                _ => return Err(Error::ArgumentNotFound("pubkey".to_string())),
            },
        };

        ctx.insert_pubkey(name, pubkey).await?;

        Ok(hashmap! {
            "pubkey".to_owned()=> ValueType::Pubkey(pubkey),
        })
    }
}
