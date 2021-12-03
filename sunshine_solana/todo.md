

graph of commands


http api




https://docs.solana.com/developing/on-chain-programs/developing-rust
Restrictions#
On-chain Rust programs support most of Rust's libstd, libcore, and liballoc, as well as many 3rd party crates.

There are some limitations since these programs run in a resource-constrained, single-threaded environment, and must be deterministic:

No access to
rand
std::fs
std::net
std::os
std::future
std::process
std::sync
std::task
std::thread
std::time

Limited access to:
std::hash
std::os

Bincode is extremely computationally expensive in both cycles and call depth and should be avoided
String formatting should be avoided since it is also computationally expensive.
No support for println!, print!, the Solana logging helpers should be used instead.
The runtime enforces a limit on the number of instructions a program can execute during the processing of one instruction. See computation budget for more information.



// Tutorials
https://imfeld.dev/writing/starting_with_solana_part01
https://github.com/paul-schaaf/solana-escrow/blob/master/program/Cargo.toml


// Projects to look at
https://github.com/heavy-duty/platform
https://github.com/heavy-duty/platform/tree/master/apps/bulldozer-programs/programs/bulldozer/src

// for Wallet
https://github.com/project-serum/spl-token-wallet
https://docs.solana.com/wallet-guide/paper-wallet


// https://github.com/solana-labs/example-helloworld

https://github.com/cryptoplease/cryptoplease-dart/blob/master/packages/solana/lib/src/crypto/ed25519_hd_keypair.dart
