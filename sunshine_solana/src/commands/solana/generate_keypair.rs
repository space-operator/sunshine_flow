use std::{collections::HashMap, sync::Arc};

use bip39::{Language, Mnemonic, Seed};
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_sdk::signature::{keypair_from_seed, Keypair};

use crate::{error::Error, ValueType};

use super::Ctx;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenerateKeypair {
    pub seed_phrase: Option<String>,
    pub passphrase: Option<String>,
    pub save: Option<Option<String>>,
}

impl GenerateKeypair {
    pub async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, ValueType>,
    ) -> Result<HashMap<String, ValueType>, Error> {
        let seed_phrase = match &self.seed_phrase {
            Some(s) => s.clone(),
            None => match inputs.remove("seed_phrase") {
                Some(ValueType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("seed_phrase".to_string())),
            },
        };
        let passphrase = match &self.passphrase {
            Some(s) => s.clone(),
            None => match inputs.remove("passphrase") {
                Some(ValueType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("passphrase".to_string())),
            },
        };
        let save: Option<String> = match self.save.clone() {
            Some(val) => val,
            None => match inputs.remove("save") {
                Some(ValueType::StringOpt(s)) => s,
                _ => return Err(Error::ArgumentNotFound("save".to_string())),
            },
        };

        let keypair = generate_keypair(&passphrase, &seed_phrase)?;

        if let Some(name) = save {
            ctx.insert_keypair(name, &keypair).await?;
        }

        Ok(hashmap! {
            "keypair".to_owned() => ValueType::Keypair(keypair.into()),
        })
    }
}

pub fn generate_keypair(passphrase: &str, seed_phrase: &str) -> Result<Keypair, Error> {
    let sanitized = seed_phrase
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ");
    let parse_language_fn = || {
        for language in &[
            Language::English,
            Language::ChineseSimplified,
            Language::ChineseTraditional,
            Language::Japanese,
            Language::Spanish,
            Language::Korean,
            Language::French,
            Language::Italian,
        ] {
            if let Ok(mnemonic) = Mnemonic::from_phrase(&sanitized, *language) {
                return Ok(mnemonic);
            }
        }
        Err(Error::CantGetMnemonicFromPhrase)
    };
    let mnemonic = parse_language_fn()?;
    let seed = Seed::new(&mnemonic, passphrase);
    keypair_from_seed(seed.as_bytes()).map_err(|e| Error::KeypairFromSeed(e.to_string()))
}
