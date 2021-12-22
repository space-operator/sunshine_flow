use serde::{Deserialize, Serialize};

pub mod simple;
pub mod solana;
//mod util;

pub enum Command {
    Simple(simple::Command),
    Solana(solana::Command),
}

impl Command {
    pub fn kind(&self) -> CommandKind {
        match self {
            Command::Simple(s) => CommandKind::Simple(s.kind()),
            Command::Solana(s) => CommandKind::Solana(s.kind()),
        }
    }
}

#[derive(Debug)]
pub enum CommandKind {
    Simple(simple::CommandKind),
    Solana(solana::CommandKind),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Config {
    Simple(simple::Command),
    Solana(solana::Kind),
}
