use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use maplit::hashmap;

use crate::{Error, Value};

pub mod branch;
pub mod http_request;
pub mod ipfs_nft_upload;
pub mod ipfs_upload;
pub mod json_extract;
pub mod json_insert;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {
    Const(Value),
    Print,
    HttpRequest(http_request::HttpRequest),
    JsonExtract(json_extract::JsonExtract),
    IpfsUpload(ipfs_upload::IpfsUpload),
    IpfsNftUpload(ipfs_nft_upload::IpfsNftUpload),
    Wait,
    Branch(branch::Branch),
    JsonInsert(json_insert::JsonInsert),
}

impl Command {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        match self {
            Command::Const(c) => Ok(hashmap! {
                "output".into() => c.clone(),
            }),
            Command::Print => {
                let arg = inputs
                    .get("print")
                    .ok_or_else(|| Error::ArgumentNotFound("print".into()))?;
                // dbg!(arg.clone());

                let arg_type = arg.kind().to_string();

                let to_print = format!("{}&&&{}", arg_type, arg);

                println!("{}", to_print);

                Ok(hashmap! {
                    "__print_output".into() => Value::String(to_print),
                })
            }
            Command::Wait => {
                if !inputs.contains_key("wait") {
                    return Err(Error::ArgumentNotFound("wait".to_string()));
                }

                let value = match inputs.remove("value") {
                    Some(v) => v,
                    None => Value::Empty,
                    _ => return Err(Error::ArgumentNotFound("value".to_string())),
                };

                Ok(hashmap! {
                    "value".into() => value
                })
            }
            Command::HttpRequest(c) => c.run(inputs).await,
            Command::JsonExtract(c) => c.run(inputs).await,
            Command::IpfsUpload(c) => c.run(inputs).await,
            Command::IpfsNftUpload(c) => c.run(inputs).await,
            Command::Branch(c) => c.run(inputs).await,
            Command::JsonInsert(c) => c.run(inputs).await,
        }
    }

    pub fn kind(&self) -> CommandKind {
        match self {
            Command::Const(_) => CommandKind::Const,
            Command::Print => CommandKind::Print,
            Command::HttpRequest(_) => CommandKind::HttpRequest,
            Command::JsonExtract(_) => CommandKind::JsonExtract,
            Command::IpfsUpload(_) => CommandKind::IpfsUpload,
            Command::IpfsNftUpload(_) => CommandKind::IpfsNftUpload,
            Command::Wait => CommandKind::Wait,
            Command::Branch(_) => CommandKind::Branch,
            Command::JsonInsert(_) => CommandKind::JsonInsert,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CommandKind {
    Const,
    Print,
    HttpRequest,
    JsonExtract,
    IpfsUpload,
    IpfsNftUpload,
    Wait,
    Branch,
    JsonInsert,
}
