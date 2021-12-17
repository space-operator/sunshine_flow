use std::{collections::HashMap, sync::Arc};

use maplit::hashmap;
use serde::{Deserialize, Serialize};

use crate::{Error, ValueType};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {}

impl Command {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, ValueType>,
    ) -> Result<HashMap<String, ValueType>, Error> {
        return Ok(inputs);
    }
}
