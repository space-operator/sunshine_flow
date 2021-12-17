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
pub struct GetBalance {
    pub name: Option<String>,
    pub pubkey: Option<Pubkey>,
}

impl GetBalance {
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

        let pubkey = ctx
            .lock()
            .unwrap()
            .get_pubkey(&name)
            .map_err(Err(Error::PubkeyDoesntExist))?;

        let balance = ctx.lock().unwrap().client.get_balance(&pubkey)?;

        // let pubkey = Pubkey::from_str(self.pubkey)?;

        Ok(hashmap! {
            "balance".to_owned()=>OutputType::Balance(balance),
        })
    }
}
