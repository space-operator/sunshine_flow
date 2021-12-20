use serde::{Deserialize, Serialize};

pub mod simple;
pub mod solana;
//mod util;

pub enum Command {
    Simple(simple::Command),
    Solana(solana::Command),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Config {
    Simple(simple::Command),
    Solana(solana::Kind),
}
