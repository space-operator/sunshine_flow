use crate::{Error, Value};

use maplit::hashmap;
use std::collections::HashMap;
use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use serde_json::Value as JsonValue;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonExtract {
    pub path: Option<String>,
    pub json: Option<Value>,
}

impl JsonExtract {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let json = match &self.json {
            Some(v) => JsonValue::try_from(v.clone())?,
            None => match inputs.remove("json") {
                Some(v) => serde_json::to_value(v)?,
                _ => return Err(Error::ArgumentNotFound("json".to_string())),
            },
        };

        let path = match &self.path {
            Some(s) => s.clone(),
            None => match inputs.remove("path") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("path".to_string())),
            },
        };

        let val = match json.pointer(&path) {
            Some(v) => Value::try_from(v.clone())?,
            None => Value::Empty,
        };

        let outputs = hashmap! {
            "value".to_owned()=> val,
        };

        Ok(outputs)
    }
}
