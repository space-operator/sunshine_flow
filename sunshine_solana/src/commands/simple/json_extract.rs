use crate::{Error, Value};

use maplit::hashmap;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use serde_json::Value as JsonValue;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonExtract {
    pub pointer: String,
    pub arg: String,
}

impl JsonExtract {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let json: JsonValue = match inputs.remove(&self.arg) {
            Some(Value::String(json)) => serde_json::from_str(&json)?,
            _ => return Err(Error::ArgumentNotFound(self.arg.to_string())),
        };

        let val = match json.pointer(&self.pointer) {
            Some(v) => match v {
                JsonValue::Null => Value::Empty,
                JsonValue::Bool(b) => Value::Bool(*b),
                JsonValue::Number(_) => Value::Empty,
                JsonValue::String(s) => Value::String(s.clone()),
                JsonValue::Array(_) => Value::Empty,
                JsonValue::Object(_) => Value::Empty,
            },
            None => Value::Empty,
        };

        let outputs = hashmap! {
            "val".to_owned()=> val,
        };

        Ok(outputs)
    }
}
