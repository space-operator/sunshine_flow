use solana_client::client_error::ClientError as SolanaClientError;
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
    ParsePubKey(solana_sdk::pubkey::ParsePubkeyError),
    #[error("solana client error: {0}")]
    SolanaClientError(SolanaClientError),
    #[error("solana signer error: {0}")]
    SolanaSignerError(SolanaSignerError),
}

impl From<sunshine_core::Error> for Error {
    fn from(err: sunshine_core::Error) -> Error {
        Error::Core(err)
    }
}

impl From<SolanaClientError> for Error {
    fn from(err: SolanaClientError) -> Error {
        Error::SolanaClientError(err)
    }
}

impl From<SolanaSignerError> for Error {
    fn from(err: SolanaSignerError) -> Error {
        Error::SolanaSignerError(err)
    }
}
