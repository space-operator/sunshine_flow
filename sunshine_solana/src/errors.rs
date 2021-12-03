use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum CustomError {
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
}
