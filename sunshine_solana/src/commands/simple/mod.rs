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
    GetPubkeyFromKeypair,
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
                    .get("p")
                    .ok_or_else(|| Error::ArgumentNotFound("p".into()))?;

                println!("{:#?}", arg);

                Ok(inputs)
            }
            Command::GetPubkeyFromKeypair => {
                let keypair = match inputs
                    .remove("keypair")
                    .ok_or_else(|| Error::ArgumentNotFound("keypair".into()))?
                {
                    Value::Keypair(kp) => kp,
                    _ => return Err(Error::ArgumentNotFound("keypair".into())),
                };

                let keypair: Keypair = keypair.into();

                Ok(hashmap! {
                    "pubkey".into() => Value::Pubkey(keypair.pubkey()),
                })
            }
        }
    }

    pub fn kind(&self) -> CommandKind {
        match self {
            Command::Const(_) => CommandKind::Const,
            Command::Print => CommandKind::Print,
            Command::GetPubkeyFromKeypair => CommandKind::GetPubkeyFromKeypair,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CommandKind {
    Const,
    Print,
    GetPubkeyFromKeypair,
}
