use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use super::Ctx;
use crate::{error::Error, Value};

pub mod create_master_edition;
pub mod create_metadata_accounts;
pub mod update_metadata_accounts;
pub mod utilize;
pub mod approve_use_authority;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Command {
    CreateMetadataAccounts(create_metadata_accounts::CreateMetadataAccounts),
    CreateMasterEdition(create_master_edition::CreateMasterEdition),
    UpdateMetadataAccounts(update_metadata_accounts::UpdateMetadataAccounts),
    Utilize(utilize::Utilize),
    ApproveUseAuthority(approve_use_authority::ApproveUseAuthority),
}

#[derive(Debug, Clone, Copy)]
pub enum CommandKind {
    CreateMetadataAccounts,
    CreateMasterEdition,
    UpdateMetadataAccount,
    Utilize,
    ApproveUseAuthority,
}

impl Command {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        match self {
            Command::CreateMetadataAccounts(k) => k.run(ctx, inputs).await,
            Command::CreateMasterEdition(k) => k.run(ctx, inputs).await,
            Command::UpdateMetadataAccounts(k) => k.run(ctx, inputs).await,
            Command::Utilize(k) => k.run(ctx, inputs).await,
            Command::ApproveUseAuthority(k) => k.run(ctx, inputs).await,
        }
    }

    pub fn kind(&self) -> CommandKind {
        match self {
            Command::CreateMetadataAccounts(_) => CommandKind::CreateMetadataAccounts,
            Command::CreateMasterEdition(_) => CommandKind::CreateMasterEdition,
            Command::UpdateMetadataAccounts(_) => CommandKind::UpdateMetadataAccount,
            Command::Utilize(_) => CommandKind::Utilize,
            Command::ApproveUseAuthority(_) => CommandKind::ApproveUseAuthority,
        }
    }
}
