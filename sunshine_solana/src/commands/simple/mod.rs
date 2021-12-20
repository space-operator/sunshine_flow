use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{Error, ValueType};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {}

impl Command {
    pub(crate) async fn run(
        &self,
        inputs: HashMap<String, ValueType>,
    ) -> Result<HashMap<String, ValueType>, Error> {
        Ok(inputs)
    }
}
