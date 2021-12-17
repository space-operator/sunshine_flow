use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
};

use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

use crate::{error::Error, OutputType};

use super::Ctx;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RequestAirdrop {
    pub name: Option<String>,
    pub pubkey: Option<Pubkey>,
    pub amount: Option<u64>,
}

impl RequestAirdrop {
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

        let amount = match &self.amount {
            Some(s) => s.clone(),
            None => match inputs.remove("amount") {
                Some(OutputType::U64(s)) => s,
                _ => return Err(Error::ArgumentNotFound("amount".to_string())),
            },
        };

        if ctx.lock().unwrap().pub_keys.contains_key(&name) {
            return Err(Error::PubkeyAlreadyExists);
        } else {
            ctx.lock().unwrap().pub_keys.insert(name.clone(), pubkey);
        }

        // add errors
        let pubkey = ctx.lock().unwrap().get_pubkey(&name)?;

        let signature = ctx
            .lock()
            .unwrap()
            .client
            .request_airdrop(&pubkey, amount)?;

        Ok(hashmap! {
            "signature".to_owned()=>OutputType::Success(signature),
        })
    }
}
