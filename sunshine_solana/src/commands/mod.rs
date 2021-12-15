use super::Msg;
use serde::{Deserialize, Serialize};

pub mod simple;
pub mod solana;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {
    Simple(simple::Command),
}
