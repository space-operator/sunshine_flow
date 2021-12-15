use std::{collections::HashMap, sync::Arc};

use maplit::hashmap;
use serde::{Deserialize, Serialize};

use crate::{Error, Msg};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {
    Print,
    Add,
    Const(Msg),
}

impl Command {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, Msg>,
    ) -> Result<HashMap<String, Msg>, Error> {
        match self {
            Command::Add => {
                let a = inputs.remove("a").unwrap();
                let b = inputs.remove("b").unwrap();

                Ok(hashmap! {
                    "res".to_owned() => a + b,
                })
            }
            Command::Print => {
                let p = inputs.remove("p").unwrap();

                println!("{:#?}", p);

                Ok(hashmap! {
                    "res".to_owned() => p,
                })
            }
            Command::Const(msg) => Ok(hashmap! {
                "res".to_owned() => *msg,
            }),
        }
    }
}
