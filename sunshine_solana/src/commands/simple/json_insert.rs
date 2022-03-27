use crate::{
    commands::solana::nft::update_metadata_accounts::MetadataAccountData, Error, NftCreator,
    NftMetadata, Value, ValueKind,
};

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
        let mut save_value: Value = Value::Cancel;

        let mut json = match &self.json {
            Some(v) => JsonValue::try_from(v.clone())?,
            None => match inputs.remove("json") {
                Some(v) => {
                    save_value = v.clone();

                    JsonValue::try_from(v)?
                }
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
        // dbg!(json.clone());
        let new_value = match save_value {
            Value::I64(_) => todo!(),
            Value::Keypair(_) => todo!(),
            Value::String(_) => todo!(),
            Value::NodeId(_) => todo!(),
            Value::DeletedNode(_) => todo!(),
            Value::Pubkey(_) => todo!(),
            Value::Success(_) => todo!(),
            Value::Balance(_) => todo!(),
            Value::U8(_) => todo!(),
            Value::U16(_) => todo!(),
            Value::U64(_) => todo!(),
            Value::F32(_) => todo!(),
            Value::F64(_) => todo!(),
            Value::Bool(_) => todo!(),
            Value::StringOpt(_) => todo!(),
            Value::Empty => todo!(),
            Value::NodeIdOpt(_) => todo!(),
            Value::NftCreators(_) => Value::NftCreators(vec![NftCreator::from(json.clone())]),
            Value::MetadataAccountData(_) => {
                Value::MetadataAccountData(MetadataAccountData::from(json.clone()))
            }
            Value::Uses(_) => todo!(),
            Value::NftMetadata(_) => Value::NftMetadata(NftMetadata::from(json.clone())),
            Value::Operator(_) => todo!(),
            Value::Json(_) => Value::Json(json.into()),
            Value::Cancel => todo!(),
        };

        let outputs = hashmap! {
            "json".to_owned() => new_value//,
        };

        Ok(outputs)
    }
}
