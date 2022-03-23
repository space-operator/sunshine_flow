use crate::{Error, Value};

use maplit::hashmap;
use std::collections::HashMap;
use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use serde_json::Value as JsonValue;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonInsert {
    pub path: Option<String>,
    pub json: Option<Value>,
    pub value: Option<Value>,
}

impl JsonInsert {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let mut json = match &self.json {
            Some(v) => JsonValue::try_from(v.clone())?,
            None => match inputs.remove("json") {
                Some(v) => JsonValue::try_from(v)?,
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

        let value = match &self.value {
            Some(v) => JsonValue::try_from(v.clone())?,
            None => match inputs.remove("value") {
                Some(v) => JsonValue::try_from(v)?,
                None => JsonValue::Null,
            },
        };

        match json.pointer_mut(&path) {
            Some(p) => {
                *p = value;
            }
            None => (),
        }

        let outputs = hashmap! {
            "json".to_owned() => Value::Json(json.into()),
        };

        Ok(outputs)
    }
}
