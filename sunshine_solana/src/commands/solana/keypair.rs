use bip39::{Language, Mnemonic, Seed};
use serde::{Deserialize, Serialize};
use solana_sdk::signature::{keypair_from_seed, Keypair};

type Error = Box<dyn std::error::Error>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenerateKeypair {
    pub seed_phrase: String,
    pub passphrase: String,
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
        Err("Can't get mnemonic from seed phrases")
    };
    let mnemonic = parse_language_fn()?;
    let seed = Seed::new(&mnemonic, passphrase);
    keypair_from_seed(seed.as_bytes())
}
