use serde::{Deserialize, Serialize};

pub mod simple;
pub mod solana;
//mod util;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {
    Simple(simple::Command),
    Solana(solana::Kind),
}
