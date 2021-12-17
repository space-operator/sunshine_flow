use std::{collections::HashMap, sync::Arc};

use maplit::hashmap;
use serde::{Deserialize, Serialize};

use crate::{Error, OutputType};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {}

impl Command {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, OutputType>,
    ) -> Result<HashMap<String, OutputType>, Error> {
        return Ok(inputs);
    }
}
