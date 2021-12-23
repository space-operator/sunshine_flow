use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use solana_sdk::signature::Signer;

use maplit::hashmap;
use solana_sdk::signature::Keypair;

use crate::{Error, Value};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {
    Const(Value),
    Print,
}

impl Command {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        match self {
            Command::Const(c) => Ok(hashmap! {
                "res".into() => c.clone(),
            }),
            Command::Print => {
                let arg = inputs
                    .get("print")
                    .ok_or_else(|| Error::ArgumentNotFound("print".into()))?;

                println!("{:#?}", arg);

                Ok(inputs)
            }
        }
    }

    pub fn kind(&self) -> CommandKind {
        match self {
            Command::Const(_) => CommandKind::Const,
            Command::Print => CommandKind::Print,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CommandKind {
    Const,
    Print,
}
