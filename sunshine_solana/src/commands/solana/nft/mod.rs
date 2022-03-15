use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use super::Ctx;
use crate::{error::Error, Value};

pub mod approve_use_authority;
pub mod arweave_bundlr;
pub mod arweave_nft_upload;
pub mod arweave_upload;
pub mod create_master_edition;
pub mod create_metadata_accounts;
pub mod get_left_uses;
pub mod update_metadata_accounts;
pub mod utilize;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Command {
    CreateMetadataAccounts(create_metadata_accounts::CreateMetadataAccounts),
    CreateMasterEdition(create_master_edition::CreateMasterEdition),
    UpdateMetadataAccounts(update_metadata_accounts::UpdateMetadataAccounts),
    Utilize(utilize::Utilize),
    ApproveUseAuthority(approve_use_authority::ApproveUseAuthority),
    GetLeftUses(get_left_uses::GetLeftUses),
    ArweaveUpload(arweave_upload::ArweaveUpload),
    ArweaveNftUpload(arweave_nft_upload::ArweaveNftUpload),
    ArweaveBundlr(arweave_bundlr::ArweaveBundlr),
}

#[derive(Debug, Clone, Copy)]
pub enum CommandKind {
    CreateMetadataAccounts,
    CreateMasterEdition,
    UpdateMetadataAccount,
    Utilize,
    ApproveUseAuthority,
    GetLeftUses,
    ArweaveUpload,
    ArweaveNftUpload,
    ArweaveBundlr,
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
            Command::GetLeftUses(k) => k.run(ctx, inputs).await,
            Command::ArweaveUpload(k) => k.run(ctx, inputs).await,
            Command::ArweaveNftUpload(k) => k.run(ctx, inputs).await,
            Command::ArweaveBundlr(k) => k.run(ctx, inputs).await,
        }
    }

    pub fn kind(&self) -> CommandKind {
        match self {
            Command::CreateMetadataAccounts(_) => CommandKind::CreateMetadataAccounts,
            Command::CreateMasterEdition(_) => CommandKind::CreateMasterEdition,
            Command::UpdateMetadataAccounts(_) => CommandKind::UpdateMetadataAccount,
            Command::Utilize(_) => CommandKind::Utilize,
            Command::ApproveUseAuthority(_) => CommandKind::ApproveUseAuthority,
            Command::GetLeftUses(_) => CommandKind::GetLeftUses,
            Command::ArweaveUpload(_) => CommandKind::ArweaveUpload,
            Command::ArweaveNftUpload(_) => CommandKind::ArweaveNftUpload,
            Command::ArweaveBundlr(_) => CommandKind::ArweaveBundlr,
        }
    }
}
