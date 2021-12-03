use serde::{Deserialize, Serialize};

use self::{
    account::CreateAccount,
    keypair::GenerateKeypair,
    token::{CreateToken, MintToken},
    transfer::Transfer,
};

pub mod account;
pub mod keypair;
pub mod token;
pub mod transfer;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {
    GenerateKeypair(String, GenerateKeypair),
    DeleteKeypair(String),
    AddPubkey(String, String),
    DeletePubkey(String),
    CreateAccount(CreateAccount),
    GetBalance(String),
    CreateToken(CreateToken),
    MintToken(MintToken),
    RequestAirdrop(String, u64),
    Transfer(Transfer),
}
