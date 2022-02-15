use crate::ValueKind;
use solana_client::client_error::ClientError as SolanaClientError;
use solana_sdk::program_error::ProgramError as SolanaProgramError;
use solana_sdk::pubkey::ParsePubkeyError;
use solana_sdk::signer::SignerError as SolanaSignerError;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("name already in use")]
    NameAlreadyInUse,
    #[error("keypair already exists in the keyring")]
    KeypairAlreadyExistsInKeyring,
    #[error("keypair doesn't exist in the keyring")]
    KeypairDoesntExist,
    #[error("public key already added")]
    PubkeyAlreadyExists,
    #[error("public key isn't added")]
    PubkeyDoesntExist,
    #[error("flow doesn't exist")]
    FlowDoesntExist,
    #[error("core error: {0}")]
    Core(sunshine_core::Error),
    #[error("argument not found: {0}")]
    ArgumentNotFound(String),
    #[error("can't get mnemonic from phrase")]
    CantGetMnemonicFromPhrase,
    #[error("failed to get keypair from seed: {0}")]
    KeypairFromSeed(String),
    #[error("failed to parse public key from string: {0}")]
    ParsePubkey(ParsePubkeyError),
    #[error("solana client error: {0}")]
    SolanaClient(SolanaClientError),
    #[error("solana signer error: {0}")]
    SolanaSigner(SolanaSignerError),
    #[error("solana error: recipient address not funded")]
    RecipientAddressNotFunded,
    #[error("solana error: unsupported recipient address: {0}")]
    UnsupportedRecipientAddress(String),
    #[error("solana error: associated token account doesn't exist")]
    AssociatedTokenAccountDoesntExist,
    #[error("solana program error: {0}")]
    SolanaProgram(SolanaProgramError),
    #[error("no context for command")]
    NoContextForCommand,
    #[error("error when trying to convert value of type {0} to type {1}")]
    ValueIntoError(ValueKind, String),
    #[error("solana airdrop failed")]
    AirdropFailed,
    #[error("io error: {0}")]
    IoError(std::io::Error),
    #[error("flow is already deployed")]
    FlowAlreadyDeployed,
    #[error("http error: {0}")]
    Http(reqwest::Error),
    #[error("base64 decode error: {0}")]
    Base64Decode(base64::DecodeError),
    #[error("invalid http method")]
    InvalidHttpMethod,
    #[error("http status code is err: {0}")]
    HttpStatus(u16),
    #[error("json error: {0}")]
    JsonError(serde_json::Error),
    #[error("failed to parse url: {0}")]
    UrlParse(String),
    #[error("arweave tx not found after submitting")]
    ArweaveTxNotFound,
    #[error("arweave upload error: {0}")]
    ArLoader(String),
}

impl From<arloader::error::Error> for Error {
    fn from(err: arloader::error::Error) -> Error {
        Error::ArLoader(err.to_string())
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Error {
        Error::UrlParse(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::JsonError(err)
    }
}

impl From<base64::DecodeError> for Error {
    fn from(err: base64::DecodeError) -> Error {
        Error::Base64Decode(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Error {
        Error::Http(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::IoError(err)
    }
}

impl From<ParsePubkeyError> for Error {
    fn from(err: ParsePubkeyError) -> Error {
        Error::ParsePubkey(err)
    }
}

impl From<SolanaProgramError> for Error {
    fn from(err: SolanaProgramError) -> Error {
        Error::SolanaProgram(err)
    }
}

impl From<sunshine_core::Error> for Error {
    fn from(err: sunshine_core::Error) -> Error {
        Error::Core(err)
    }
}

impl From<SolanaClientError> for Error {
    fn from(err: SolanaClientError) -> Error {
        Error::SolanaClient(err)
    }
}

impl From<SolanaSignerError> for Error {
    fn from(err: SolanaSignerError) -> Error {
        Error::SolanaSigner(err)
    }
}
