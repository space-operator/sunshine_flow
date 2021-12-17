use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
};

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
        ctx: Arc<Mutex<Ctx>>,
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
            Some(s) => s.clone(),
            None => match inputs.remove("pubkey") {
                Some(ValueType::Pubkey(s)) => s,
                _ => return Err(Error::ArgumentNotFound("pubkey".to_string())),
            },
        };

        if ctx.lock().unwrap().pub_keys.contains_key(&name) {
            return Err(Error::PubkeyAlreadyExists);
        } else {
            ctx.lock().unwrap().pub_keys.insert(name.clone(), pubkey);
        }

        // let pubkey = Pubkey::from_str(self.pubkey)?;

        Ok(hashmap! {
            "pubkey".to_owned()=>ValueType::Pubkey(pubkey),
        })
    }
}
