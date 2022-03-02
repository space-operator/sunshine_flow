use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use solana_sdk::signature::Signer;

use maplit::hashmap;
use solana_sdk::signature::Keypair;

use crate::{Error, Value};

pub mod http_request;
pub mod ipfs_upload;
pub mod json_extract;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {
    Const(Value),
    Print,
    HttpRequest(http_request::HttpRequest),
    JsonExtract(json_extract::JsonExtract),
    IpfsUpload(ipfs_upload::IpfsUpload),
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

                let to_print = format!("{:#?}", arg);

                println!("{}", to_print);

                Ok(hashmap! {
                    "__print_output".into() => Value::String(to_print),
                })
            }
            Command::HttpRequest(c) => c.run(inputs).await,
            Command::JsonExtract(c) => c.run(inputs).await,
            Command::IpfsUpload(c) => c.run(inputs).await,
        }
    }

    pub fn kind(&self) -> CommandKind {
        match self {
            Command::Const(_) => CommandKind::Const,
            Command::Print => CommandKind::Print,
            Command::HttpRequest(_) => CommandKind::HttpRequest,
            Command::JsonExtract(_) => CommandKind::JsonExtract,
            Command::IpfsUpload(_) => CommandKind::IpfsUpload,
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
}
