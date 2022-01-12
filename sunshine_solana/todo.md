
**NFT Standards v1.1**
    https://docs.metaplex.com/token-metadata/v1.1.0/overview

**mpl_token_metadata crate**
https://metaplex-foundation.github.io/metaplex-program-library/mpl_token_metadata/state/index.html


**collections**
https://docs.metaplex.com/token-metadata/v1.1.0/specification#on-chain-representation-of-a-collection

**follow metadata improvement progress**
https://github.com/orgs/metaplex-foundation/projects/1/views/25


**code location**
    https://github.com/metaplex-foundation/metaplex-program-library/tree/master/token-metadata/program


**Register Token**
https://github.com/solana-labs/token-list#adding-new-token

**Example Mainnet NFT pubkey**
7C4cjqUxd38cGsemdxjyNcTxfdFeh1CCMELy4ih5ckYg


**Metadata**

Deep Dive: Basic Single Item Auction End To End
https://docs.metaplex.com/architecture/basic_flow

example 
 metaplex-program-library/blob/master/token-metadata/test/src/main.rs

    
    https://github.com/metaplex-foundation/metaplex/tree/master/rust/token-metadata/program/tests
    https://github.com/metaplex-foundation/metaplex-program-library/tree/master/token-metadata/program
    Newer repo
        https://github.com/metaplex-foundation/metaplex-program-library/tree/master/token-metadata/program
        https://github.com/metaplex-foundation/metaplex/blob/master/rust/token-metadata/test/src/main.rs

    https://docs.rs/metaplex-token-metadata/0.0.1/metaplex_token_metadata/index.html




**Third party Library to check**
https://github.com/samuelvanderwaal/metaboss

**Filed Issues**
Closed - https://github.com/metaplex-foundation/metaplex/issues/1448

**Previous Issues**
Adding NFT support to the Explorer #20009
https://github.com/solana-labs/solana/issues/19516



Master Edition
1. User creates a new Metadata for their mint with create_metadata_accounts() which makes new Metadata
2. User wishes their mint to be a master edition and ensures that there is only required supply of one in the mint.
3. User requests the program to designate create_master_edition() on their metadata, which creates new MasterEdition which for this example we will say has an unlimited supply. As part of the arguments to the function the user is required to make a new mint called the Printing mint over which they have minting authority that they tell the contract about and that the contract stores on the MasterEdition.

4. User mints a token from the Printing mint and gives it to their friend.
5. Their friend creates a new mint with supply 1 and calls mint_new_edition_from_master_edition_via_token(), which creates for them new Metadata and Edition records signifying this mint as an Edition child of the master edition original.




https://github.com/metaplex-foundation/metaplex/tree/master/rust/nft-candy-machine-v2/src


    https://docs.metaplex.com/candy-machine-v2/creating-candy-machine
    https://github.com/metaplex-foundation/metaplex/blob/master/js/packages/cli/src/candy-machine-v2-cli.ts

5.
6. mint token

7. Withdraw Rent
https://docs.metaplex.com/candy-machine-v2/withdraw

8. Update Metadata
https://github.com/metaplex-foundation/metaplex-program-library/blob/7ec8bec69d3eb5afc78e8f03d57bad3237204f06/token-metadata/test/src/main.rs#L470
https://docs.metaplex.com/candy-machine-v2/update

8. Approve Use Authority
https://metaplex-foundation.github.io/metaplex-program-library/mpl_token_metadata/instruction/fn.approve_use_authority.html
8. revoke utilize authority

9. Print Copies -mint_new_edition_from_master_edition_via_token

10. utilize
Utilize or Use an NFT , burns the NFT and returns the lamports to the update authority if the use method is burn and its out of uses. Use Authority can be the Holder of the NFT, or a Delegated Use Authority.
https://metaplex-foundation.github.io/metaplex-program-library/mpl_token_metadata/instruction/fn.utilize.html



11. primary sale happened

12. fireball
https://github.com/metaplex-foundation/metaplex/pull/1125

Fair Launch

Create Store
https://github.com/metaplex-foundation/metaplex/blob/master/rust/metaplex/program/tests/create_store.rs




single flat error type> error handling
reading arguments
convert command implementations
correctness of types
triggers
pause/wait
multiple outputs to one node





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
