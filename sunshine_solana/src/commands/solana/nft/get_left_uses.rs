use std::{collections::HashMap, sync::Arc};

use super::super::Ctx;
use borsh::BorshDeserialize;
use maplit::hashmap;
use mpl_token_metadata::state::Metadata;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

use sunshine_core::msg::NodeId;

use crate::{Error, Value};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetLeftUses {
    pub mint_account: Option<NodeId>,
}

impl GetLeftUses {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let mint_account = match self.mint_account {
            Some(s) => ctx.get_pubkey_by_id(s).await?,
            None => match inputs.remove("mint_account") {
                Some(Value::NodeId(id)) => ctx.get_pubkey_by_id(id).await?,
                Some(v) => v.try_into()?,
                _ => return Err(Error::ArgumentNotFound("mint_account".to_string())),
            },
        };

        let program_id = mpl_token_metadata::id();

        let metadata_seeds = &[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            &program_id.as_ref(),
            mint_account.as_ref(),
        ];

        let (metadata_pubkey, _) = Pubkey::find_program_address(metadata_seeds, &program_id);

        let account_data = ctx.client.get_account_data(&metadata_pubkey)?;

        let mut account_data_ptr = account_data.as_slice();

        let metadata = Metadata::deserialize(&mut account_data_ptr)?;

        let left_uses = match metadata.uses {
            Some(uses) => Value::U64(uses.remaining),
            None => Value::Empty,
        };

        let outputs = hashmap! {
            "left_uses".to_owned()=> left_uses,
        };

        Ok(outputs)
    }
}
