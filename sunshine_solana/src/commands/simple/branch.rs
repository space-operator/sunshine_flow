use crate::{Error, NftMetadata, Value};

use maplit::hashmap;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use reqwest::{Client, Method};

use reqwest::multipart::{Form, Part};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Branch {
    pub operator: Option<Operator>,
    pub a: Option<Value>,
    pub b: Option<Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, parse_display::Display)]
pub enum Operator {
    Eq,
    NotEq,
    Greater,
    Less,
    GreaterEq,
    LessEq,
}

impl Branch {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let operator = match &self.operator {
            Some(s) => s.clone(),
            None => match inputs.remove("operator") {
                Some(Value::Operator(p)) => p,
                _ => return Err(Error::ArgumentNotFound("operator".to_string())),
            },
        };

        let a = match &self.a {
            Some(s) => s.clone(),
            None => match inputs.remove("a") {
                Some(v) => v,
                _ => return Err(Error::ArgumentNotFound("a".to_string())),
            },
        };

        let mut b = match &self.b {
            Some(s) => s.clone(),
            None => match inputs.remove("b") {
                Some(v) => v,
                _ => return Err(Error::ArgumentNotFound("b".to_string())),
            },
        };

        let mut outputs = HashMap::new();

        let res = match (a, operator, b) {
            (Value::U64(a), Operator::Eq, Value::U64(b)) => a == b,
            (Value::U64(a), Operator::NotEq, Value::U64(b)) => a != b,
            (Value::U64(a), Operator::Greater, Value::U64(b)) => a > b,
            (Value::U64(a), Operator::Less, Value::U64(b)) => a < b,
            (Value::U64(a), Operator::GreaterEq, Value::U64(b)) => a >= b,
            (Value::U64(a), Operator::LessEq, Value::U64(b)) => a <= b,
            //
            (Value::F64(a), Operator::Eq, Value::F64(b)) => a == b,
            (Value::F64(a), Operator::NotEq, Value::F64(b)) => a != b,
            (Value::F64(a), Operator::Greater, Value::F64(b)) => a > b,
            (Value::F64(a), Operator::Less, Value::F64(b)) => a < b,
            (Value::F64(a), Operator::GreaterEq, Value::F64(b)) => a >= b,
            (Value::F64(a), Operator::LessEq, Value::F64(b)) => a <= b,
            (a, operator, b) => {
                return Err(Error::ComparisonError {
                    a: a.kind(),
                    operator,
                    b: b.kind(),
                })
            }
        };

        if res {
            outputs.insert("__true_branch".to_owned(), Value::Empty);
        } else {
            outputs.insert("__false_branch".to_owned(), Value::Empty);
        }

        Ok(outputs)
    }
}
