use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

use crate::{error::Error, ValueType};

use super::Ctx;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetBalance {
    pub name: Option<String>,
    pub pubkey: Option<Pubkey>,
}

impl GetBalance {
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

        let pubkey = ctx
            .lock()
            .unwrap()
            .get_pubkey(&name)
            .map_err(Err(Error::PubkeyDoesntExist))?;

        let balance = ctx.lock().unwrap().client.get_balance(&pubkey)?;

        // let pubkey = Pubkey::from_str(self.pubkey)?;

        Ok(hashmap! {
            "balance".to_owned()=>ValueType::Balance(balance),
        })
    }
}
