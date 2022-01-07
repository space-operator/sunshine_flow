use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use super::Ctx;
use crate::{error::Error, Value};

pub mod create_metadata_accounts;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Command {
    CreateMetadataAccounts(create_metadata_accounts::CreateMetadataAccounts),
}

#[derive(Debug, Clone, Copy)]
pub enum CommandKind {
    CreateMetadataAccounts,
}

impl Command {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        match self {
            Command::CreateMetadataAccounts(k) => k.run(ctx, inputs).await,
        }
    }

    pub fn kind(&self) -> CommandKind {
        match self {
            Command::CreateMetadataAccounts(_) => CommandKind::CreateMetadataAccounts,
        }
    }
}
