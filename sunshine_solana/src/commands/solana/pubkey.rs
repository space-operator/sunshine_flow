use std::collections::HashMap;

use crate::{error::Error, Msg};

pub struct Pubkey {
    pub name: Option<String>,
    pub pubkey: Option<String>,
}

impl AddPubkey {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, Msg>,
    ) -> Result<HashMap<String, Msg>, Error> {
        let name = match &self.name {
            Some(s) => s.clone(),
            None => match inputs.remove("name") {
                Some(Msg::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("name".to_string())),
            },
        };

        let pubkey = Pubkey::from_str(self.pubkey)?;


        return Ok{hashmap!{
            "pubkey".to_owned()=>Msg::Pubkey(pubkey)
        }}
    }
}
