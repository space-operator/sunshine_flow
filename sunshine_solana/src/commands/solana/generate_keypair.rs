use std::collections::HashMap;

use bip39::{Language, Mnemonic, Seed};
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use solana_sdk::signature::{keypair_from_seed, Keypair};

use crate::{error::Error, OutputType};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenerateKeypair {
    pub seed_phrase: Option<String>,
    pub passphrase: Option<String>,
}

impl GenerateKeypair {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, OutputType>,
    ) -> Result<HashMap<String, OutputType>, Error> {
        let seed_phrase = match &self.seed_phrase {
            Some(s) => s.clone(),
            None => match inputs.remove("seed_phrase") {
                Some(OutputType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("seed_phrase".to_string())),
            },
        };
        let passphrase = match &self.passphrase {
            Some(s) => s.clone(),
            None => match inputs.remove("passphrase") {
                Some(OutputType::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("passphrase".to_string())),
            },
        };

        let keypair = generate_keypair(&passphrase, &seed_phrase)?;

        return Ok(hashmap! {
            "keypair".to_owned() => OutputType::Keypair(keypair),
        });
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
