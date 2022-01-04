use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use super::Ctx;
use crate::{error::Error, Value};

pub mod initialize_candy_machine;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Command {
    InitializeCandyMachine(initialize_candy_machine::InitializeCandyMachine),
}

#[derive(Debug, Clone, Copy)]
pub enum CommandKind {
    InitializeCandyMachine,
}

impl Command {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        match self {
            Command::InitializeCandyMachine(k) => k.run(self.ctx.clone(), inputs).await,
        }
    }

    pub fn kind(&self) -> CommandKind {
        match self {
            Command::InitializeCandyMachine(_) => CommandKind::InitializeCandyMachine,
        }
    }
}
