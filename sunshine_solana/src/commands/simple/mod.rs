use std::{collections::HashMap, sync::Arc};

use maplit::hashmap;
use serde::{Deserialize, Serialize};

use crate::{Error, Msg};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {}

impl Command {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, Msg>,
    ) -> Result<HashMap<String, Msg>, Error> {
        return Ok(inputs);
    }
}
