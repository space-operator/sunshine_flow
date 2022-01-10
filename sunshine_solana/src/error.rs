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
