use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

use crate::{error::Error, OutputType};

use super::Ctx;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeletePubkey {
    pub name: Option<String>,
    pub pubkey: Option<Pubkey>,
}

impl DeletePubkey {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Mutex<Ctx>>,
        mut inputs: HashMap<String, OutputType>,
    ) -> Result<HashMap<String, OutputType>, Error> {
        let name = match &self.name {
            Some(s) => s.clone(),
            None => match inputs.remove("name") {
                Some(OutputType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("name".to_string())),
            },
        };

        let pubkey = match &self.pubkey {
            Some(s) => s.clone(),
            None => match inputs.remove("pubkey") {
                Some(OutputType::Pubkey(s)) => s,
                _ => return Err(Error::ArgumentNotFound("pubkey".to_string())),
            },
        };

        if ctx.lock().unwrap().pub_keys.contains_key(&name) {
            ctx.lock().unwrap().pub_keys.remove(&name);
            Ok(hashmap! {
                 "pubkey".to_owned() => OutputType::Pubkey(pubkey),
            })
        } else {
            return Err(Error::PubkeyDoesntExist);
        }
    }
}
