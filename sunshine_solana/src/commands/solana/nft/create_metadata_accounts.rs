use std::{collections::HashMap, sync::Arc};

use super::super::Ctx;
use maplit::hashmap;
use mpl_token_metadata::state::{Collection, Creator, UseMethod, Uses};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

use sunshine_core::msg::NodeId;

use crate::{
    commands::solana::instructions::execute, CommandResult, Error, NftCreator, NftMetadata, Value,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateMetadataAccounts {
    pub token: Option<NodeId>,
    pub token_authority: Option<NodeId>,
    pub fee_payer: Option<NodeId>,        // keypair
    pub update_authority: Option<NodeId>, // keypair
    pub uri: Option<String>,
    pub metadata: Option<NftMetadata>,
    pub update_authority_is_signer: Option<bool>,
    pub is_mutable: Option<bool>,
    pub uses: Option<Option<NftUses>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NftCollection {
    pub verified: bool,
    pub key: Pubkey,
}

impl Into<Collection> for NftCollection {
    fn into(self) -> Collection {
        Collection {
            verified: self.verified,
            key: self.key,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NftUses {
    pub use_method: NftUseMethod,
    pub remaining: u64,
    pub total: u64,
}

impl Into<Uses> for NftUses {
    fn into(self) -> Uses {
        Uses {
            use_method: self.use_method.into(),
            remaining: self.remaining,
            total: self.total,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NftUseMethod {
    Burn,
    Single,
    Multiple,
}

impl Into<UseMethod> for NftUseMethod {
    fn into(self) -> UseMethod {
        match self {
            NftUseMethod::Burn => UseMethod::Burn,
            NftUseMethod::Single => UseMethod::Single,
            NftUseMethod::Multiple => UseMethod::Multiple,
        }
    }
}

impl CreateMetadataAccounts {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let token = match self.token {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("token") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("token".to_string())),
            },
        };

        let token_authority = match self.token_authority {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("token_authority") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("token_authority".to_string())),
            },
        };

        let fee_payer = match self.fee_payer {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("fee_payer") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("fee_payer".to_string())),
            },
        };

        let update_authority = match self.update_authority {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("update_authority") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("update_authority".to_string())),
            },
        };

        let mut metadata = match &self.metadata {
            Some(s) => s.clone(),
            None => match inputs.remove("metadata") {
                Some(Value::NftMetadata(s)) => s,
                _ => return Err(Error::ArgumentNotFound("metadata".to_string())),
            },
        };

        let name = metadata.name;

        let symbol = metadata.symbol;

        let uri = match &self.uri {
            Some(s) => s.clone(),
            None => match inputs.remove("uri") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("uri".to_string())),
            },
        };

        let seller_fee_basis_points = metadata.seller_fee_basis_points;

        let update_authority_is_signer = match self.update_authority_is_signer {
            Some(s) => s,
            None => match inputs.remove("update_authority_is_signer") {
                Some(Value::Bool(s)) => s,
                None => true,
                _ => {
                    return Err(Error::ArgumentNotFound(
                        "update_authority_is_signer".to_string(),
                    ))
                }
            },
        };

        let is_mutable = match self.is_mutable {
            Some(s) => s,
            None => match inputs.remove("is_mutable") {
                Some(Value::Bool(s)) => s,
                None => false,
                _ => return Err(Error::ArgumentNotFound("is_mutable".to_string())),
            },
        };

        let uses = match self.uses.clone() {
            Some(uses) => uses,
            None => match inputs.remove("uses") {
                Some(Value::Uses(uses)) => Some(uses),
                None => None,
                Some(Value::Empty) => None,
                _ => return Err(Error::ArgumentNotFound("uses".to_string())),
            },
        };

        let collection = metadata.collection.map(|c| Collection {
            verified: c.verified,
            key: c.key,
        });

        let creators = metadata
            .properties
            .map(|props| {
                props
                    .creators
                    .map(|creators| {
                        if creators.is_empty() {
                            return None;
                        }

                        Some(creators.into_iter().map(NftCreator::into).collect())
                    })
                    .flatten()
            })
            .flatten();

        let program_id = mpl_token_metadata::id();

        let metadata_seeds = &[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            &program_id.as_ref(),
            token.as_ref(),
        ];

        let (metadata_pubkey, _) = Pubkey::find_program_address(metadata_seeds, &program_id);

        let (minimum_balance_for_rent_exemption, instructions) = command_create_metadata_accounts(
            &ctx.client,
            metadata_pubkey,
            token,
            token_authority,
            fee_payer.pubkey(),
            update_authority.pubkey(),
            name,
            symbol,
            uri,
            creators,
            seller_fee_basis_points,
            update_authority_is_signer,
            is_mutable,
            collection.map(Into::into),
            uses.map(Into::into),
        )?;

        let fee_payer_pubkey = fee_payer.pubkey();

        let signers: Vec<&dyn Signer> = vec![&update_authority, &fee_payer];

        let res = execute(
            &signers,
            &ctx.client,
            &fee_payer_pubkey,
            &instructions,
            minimum_balance_for_rent_exemption,
        );

        let signature = res?;

        let outputs = hashmap! {
            "signature".to_owned()=>Value::Success(signature),
            "fee_payer".to_owned()=>Value::Keypair(fee_payer.into()),
            "token".to_owned()=>Value::Pubkey(token.into()),
            "metadata_pubkey".to_owned()=>Value::Pubkey(metadata_pubkey.into()),
        };

        Ok(outputs)
    }
}

pub fn command_create_metadata_accounts(
    rpc_client: &RpcClient,
    metadata_pubkey: Pubkey,
    mint: Pubkey,
    mint_authority: Pubkey,
    payer: Pubkey,
    update_authority: Pubkey,
    name: String,
    symbol: String,
    uri: String,
    creators: Option<Vec<Creator>>,
    seller_fee_basis_points: u16,
    update_authority_is_signer: bool,
    is_mutable: bool,
    collection: Option<Collection>,
    uses: Option<Uses>,
) -> CommandResult {
    let minimum_balance_for_rent_exemption =
        rpc_client.get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            mpl_token_metadata::state::Metadata,
        >())?;

    let instructions = vec![
        mpl_token_metadata::instruction::create_metadata_accounts_v2(
            mpl_token_metadata::id(),
            metadata_pubkey,
            mint,
            mint_authority,
            payer,
            update_authority,
            name,
            symbol,
            uri,
            creators,
            seller_fee_basis_points,
            update_authority_is_signer,
            is_mutable,
            collection,
            uses,
        ),
    ];

    Ok((minimum_balance_for_rent_exemption, instructions))
}
