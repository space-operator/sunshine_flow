// // https://docs.rs/solana-clap-utils/1.8.4/solana_clap_utils/
// // https://github.com/solana-labs/solana/tree/master/cli/src
// // https://docs.solana.com/developing/clients/jsonrpc-api
// // https://docs.rs/solana-client/1.8.4/solana_client/index.html

// // https://explorer.solana.com/address/Ew35LRHATB4w9KsS2CJNMVCgV9ajaP68WF627Fwwzymb?cluster=devnet

// // https://github.com/solana-labs/solana-program-library/blob/master/token/cli/src/main.rs

// // https://github.com/solana-labs/solana-program-library

// // https://blog.logrocket.com/how-to-create-solana-wallet-go/
// //https://spl.solana.com/token#creating-a-new-token-type
// //https://paulx.dev/blog/2021/01/14/programming-on-solana-an-introduction/#token-ownership
// //https://docs.rs/spl-associated-token-account/1.0.3/spl_associated_token_account/

// use std::str::FromStr;

// use bip39::{Language, Mnemonic, Seed};
// use solana_client::rpc_client::RpcClient;
// use solana_sdk::instruction::Instruction;
// use solana_sdk::message::Message;
// use solana_sdk::program_pack::Pack;
// use solana_sdk::pubkey::Pubkey;
// use solana_sdk::signature::keypair_from_seed;
// use solana_sdk::signer::keypair::Keypair;
// use solana_sdk::signer::Signer;
// use solana_sdk::system_instruction;
// use solana_sdk::system_program;
// use solana_sdk::transaction::Transaction;
// use spl_token::instruction::mint_to_checked;
// use spl_token::instruction::transfer_checked;
// use spl_token::state::Mint;
// use std::collections::HashMap;
// use thiserror::Error as ThisError;

// enum Command {
//     GenerateKeypair(String, GenerateKeypair),
//     DeleteKeypair(String),
//     AddPubkey(String, String),
//     DeletePubkey(String),
//     CreateAccount(CreateAccount),
//     GetBalance(String),
//     CreateToken(CreateToken),
//     RequestAirdrop(String, u64),
//     MintToken,
//     Transfer,
// }

// #[derive(Debug)]
// struct GenerateKeypair {
//     seed_phrase: String,
//     passphrase: String,
// }

// fn generate_keypair(passphrase: &str, seed_phrase: &str) -> Result<Keypair, Error> {
//     let sanitized = seed_phrase
//         .split_whitespace()
//         .collect::<Vec<&str>>()
//         .join(" ");
//     let parse_language_fn = || {
//         for language in &[
//             Language::English,
//             Language::ChineseSimplified,
//             Language::ChineseTraditional,
//             Language::Japanese,
//             Language::Spanish,
//             Language::Korean,
//             Language::French,
//             Language::Italian,
//         ] {
//             if let Ok(mnemonic) = Mnemonic::from_phrase(&sanitized, *language) {
//                 return Ok(mnemonic);
//             }
//         }
//         Err("Can't get mnemonic from seed phrases")
//     };
//     let mnemonic = parse_language_fn()?;
//     let seed = Seed::new(&mnemonic, passphrase);
//     keypair_from_seed(seed.as_bytes())
// }

// #[derive(Debug)]
// enum CommandResponse {
//     Success,
//     Balance(u64),
// }

// #[derive(ThisError, Debug)]
// enum CustomError {
//     #[error("keypair already exists in the keyring")]
//     KeypairAlreadyExistsInKeyring,
//     #[error("keypair doesn't exist in the keyring")]
//     KeypairDoesntExist,
//     #[error("public key already added")]
//     PubkeyAlreadyExists,
//     #[error("public key isn't added")]
//     PubkeyDoesntExist,
// }

// struct State {
//     client: RpcClient,
//     keyring: HashMap<String, Keypair>,
//     pub_keys: HashMap<String, Pubkey>,
// }

// impl State {
//     fn new(cfg: Config) -> Result<State, Error> {
//         let keyring = cfg
//             .keyring
//             .into_iter()
//             .map(|(name, gen_keypair)| {
//                 Ok((
//                     name,
//                     generate_keypair(&gen_keypair.passphrase, &gen_keypair.seed_phrase)?,
//                 ))
//             })
//             .collect::<Result<HashMap<String, Keypair>, Error>>()?;

//         let pub_keys = cfg
//             .pub_keys
//             .into_iter()
//             .map(|(name, pubkey)| Ok((name, Pubkey::from_str(&pubkey)?)))
//             .collect::<Result<HashMap<String, Pubkey>, Error>>()?;

//         Ok(State {
//             client: RpcClient::new(cfg.url),
//             keyring,
//             pub_keys,
//         })
//     }

//     fn get_keypair(&self, name: &str) -> Result<&Keypair, Error> {
//         self.keyring
//             .get(name)
//             .ok_or(Box::new(CustomError::KeypairDoesntExist))
//     }

//     fn get_pubkey(&self, name: &str) -> Result<&Pubkey, Error> {
//         self.pub_keys
//             .get(name)
//             .ok_or(Box::new(CustomError::PubkeyDoesntExist))
//     }

//     fn run_command(&mut self, cmd: Command) -> Result<CommandResponse, Error> {
//         match cmd {
//             Command::GenerateKeypair(name, gen_keypair) => {
//                 if self.keyring.contains_key(&name) {
//                     return Err(Box::new(CustomError::KeypairAlreadyExistsInKeyring));
//                 }

//                 if self.pub_keys.contains_key(&name) {
//                     return Err(Box::new(CustomError::PubkeyAlreadyExists));
//                 }

//                 let keypair = generate_keypair(&gen_keypair.passphrase, &gen_keypair.seed_phrase)?;

//                 self.pub_keys.insert(name.clone(), keypair.pubkey());
//                 self.keyring.insert(name, keypair);

//                 Ok(CommandResponse::Success)
//             }
//             Command::DeleteKeypair(name) => {
//                 if self.keyring.remove(&name).is_none() {
//                     return Err(Box::new(CustomError::KeypairDoesntExist));
//                 }

//                 if self.pub_keys.remove(&name).is_none() {
//                     return Err(Box::new(CustomError::PubkeyDoesntExist));
//                 }

//                 Ok(CommandResponse::Success)
//             }
//             Command::AddPubkey(name, pubkey) => {
//                 if self.pub_keys.contains_key(&name) {
//                     return Err(Box::new(CustomError::PubkeyAlreadyExists));
//                 }

//                 let pubkey = Pubkey::from_str(&pubkey)?;

//                 self.pub_keys.insert(name, pubkey);

//                 Ok(CommandResponse::Success)
//             }
//             Command::DeletePubkey(name) => {
//                 if self.pub_keys.remove(&name).is_none() {
//                     return Err(Box::new(CustomError::PubkeyDoesntExist));
//                 }

//                 Ok(CommandResponse::Success)
//             }
//             Command::CreateAccount(create_account) => {
//                 let owner = self.get_pubkey(&create_account.owner)?;
//                 let fee_payer = self.get_keypair(&create_account.fee_payer)?;
//                 let token = self.get_pubkey(&create_account.token)?;
//                 let account = match create_account.account {
//                     Some(ref account) => Some(self.get_keypair(account)?),
//                     None => None,
//                 };

//                 let (minimum_balance_for_rent_exemption, instructions) = command_create_account(
//                     &self.client,
//                     fee_payer.pubkey(),
//                     *token,
//                     *owner,
//                     account.map(|a| a.pubkey()),
//                 )
//                 .unwrap();

//                 let mut signers: Vec<&dyn Signer> = vec![fee_payer];

//                 if let Some(account) = account {
//                     signers.push(account);
//                 };

//                 execute_instructions(
//                     &signers,
//                     &self.client,
//                     &fee_payer.pubkey(),
//                     &instructions,
//                     minimum_balance_for_rent_exemption,
//                 )?;

//                 Ok(CommandResponse::Success)
//             }
//             Command::GetBalance(name) => {
//                 let pubkey = self.get_pubkey(&name)?;

//                 let balance = self.client.get_balance(pubkey)?;

//                 Ok(CommandResponse::Balance(balance))
//             }
//             Command::CreateToken(create_token) => {
//                 let fee_payer = self.get_keypair(&create_token.fee_payer)?;
//                 let authority = self.get_keypair(&create_token.authority)?;
//                 let token = self.get_keypair(&create_token.token)?;

//                 let (minimum_balance_for_rent_exemption, instructions) = command_create_token(
//                     &self.client,
//                     &fee_payer.pubkey(),
//                     create_token.decimals,
//                     &token.pubkey(),
//                     authority.pubkey(),
//                     &create_token.memo,
//                 )
//                 .unwrap();

//                 let signers: Vec<&dyn Signer> = vec![authority, fee_payer, token];

//                 execute_instructions(
//                     &signers,
//                     &self.client,
//                     &fee_payer.pubkey(),
//                     &instructions,
//                     minimum_balance_for_rent_exemption,
//                 )?;

//                 Ok(CommandResponse::Success)
//             }
//             Command::RequestAirdrop(name, amount) => {
//                 let pubkey = self.get_pubkey(&name)?;

//                 self.client.request_airdrop(pubkey, amount)?;

//                 Ok(CommandResponse::Success)
//             }
//             Command::MintToken => todo!(),
//             Command::Transfer => todo!(),
//         }
//     }
// }

// #[derive(Debug)]
// struct CreateToken {
//     fee_payer: String,
//     decimals: u8,
//     authority: String,
//     token: String,
//     memo: String,
// }

// #[derive(Debug)]
// struct CreateAccount {
//     owner: String,
//     fee_payer: String,
//     token: String,
//     account: Option<String>,
// }

// #[derive(Debug)]
// struct Config {
//     url: String,
//     keyring: HashMap<String, GenerateKeypair>,
//     pub_keys: HashMap<String, String>,
// }

// fn main() {
//     let mut state = State::new(Config {
//         url: "https://api.devnet.solana.com".to_owned(),
//         keyring: HashMap::new(),
//         pub_keys: HashMap::new(),
//     })
//     .unwrap();

//     state
//         .run_command(Command::GenerateKeypair(
//             "me".to_owned(),
//             GenerateKeypair {
//                 passphrase: "me123".into(),
//                 seed_phrase:
//                     "beach soldier piano click essay sock stable cover angle wear aunt advice"
//                         .into(),
//             },
//         ))
//         .unwrap();

//     state
//         .run_command(Command::RequestAirdrop("me".to_owned(), 1_000_000_000))
//         .unwrap();

//     state
//         .run_command(Command::GenerateKeypair(
//             "good_token".into(),
//             GenerateKeypair {
//                 passphrase: "coinpass123".into(),
//                 seed_phrase: "guard gun term bless spare iron miss flee solid forum bring will"
//                     .into(),
//             },
//         ))
//         .unwrap();

//     println!("{}", state.pub_keys.get("good_token").unwrap());

//     state
//         .run_command(Command::CreateToken(CreateToken {
//             fee_payer: "me".into(),
//             decimals: 4,
//             authority: "me".into(),
//             token: "good_token".into(),
//             memo: "CREATING GOOD TOKEN".into(),
//         }))
//         .unwrap();

//     println!("{}", state.pub_keys.get("good_token").unwrap());

//     //https://github.com/solana-labs/token-list
//     //https://github.com/solana-labs/solana/blob/b8ac6c1889d93e10967ddac850f9dd8c5b1c5c95/explorer/src/pages/AccountDetailsPage.tsx
//     // 1. wallet
//     //      add accounts
//     // 2. create token account

//     //let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
//     //panic!("{}", mnemonic.phrase());
//     /*
//     let seed_phrase = "beach soldier piano click essay sock stable cover angle wear aunt advice";
//     let keypair = generate_keypair("", seed_phrase).unwrap();
//     let client = RpcClient::new("https://api.devnet.solana.com".to_owned());
//     let user = keypair.pubkey();
//     let seed_phrase = "guard gun term bless spare iron miss flee solid forum bring will";
//     let token_keypair = generate_keypair("", seed_phrase).unwrap();
//     let token = token_keypair.pubkey();
//     println!("Creating token {token}");
//     let seed_phrase = "risk foster path suit lecture fit ancient allow major reward open favorite";
//     let custom_token_account_keypair = generate_keypair("", seed_phrase).unwrap();
//     let custom_token_account = custom_token_account_keypair.pubkey();
//     println!("custom token account1: {custom_token_account}");
//     let seed_phrase =
//         "property space future road athlete various frame doll evolve stuff aim hidden";
//     let custom_token_account_keypair2 = generate_keypair("", seed_phrase).unwrap();
//     let custom_token_account2 = custom_token_account_keypair2.pubkey();
//     println!("custom token account2: {custom_token_account2}");
//     println!("creating account: {custom_token_account2}");
//     let (minimum_balance_for_rent_exemption, instructions) =
//         command_create_account(&client, user, token, user, Some(custom_token_account2)).unwrap();
//     let signers: Vec<&dyn Signer> = vec![&keypair, &custom_token_account_keypair2];
//     execute_instructions(
//         &signers,
//         &client,
//         &user,
//         &instructions,
//         minimum_balance_for_rent_exemption,
//     );
//     println!("Minting token {token}");
//     let (minimum_balance_for_rent_exemption, instructions) =
//         command_mint(&client, token, 120.0, custom_token_account, user).unwrap();
//     let signers: Vec<&dyn Signer> = vec![&keypair, &token_keypair];
//     execute_instructions(
//         &signers,
//         &client,
//         &user,
//         &instructions,
//         minimum_balance_for_rent_exemption,
//     );
//     println!("sending money from {custom_token_account} to {custom_token_account2}");
//     let (minimum_balance_for_rent_exemption, instructions) = command_transfer(
//         &client,
//         &custom_token_account,
//         token,
//         24.0,
//         custom_token_account2,
//         Some(custom_token_account),
//         user,
//         true,
//         true,
//         Some("SENDING MONEY TO SECOND ACCOUNT".to_owned()),
//     )
//     .unwrap();
//     let signers: Vec<&dyn Signer> = vec![&keypair, &custom_token_account_keypair];
//     execute_instructions(
//         &signers,
//         &client,
//         &user,
//         &instructions,
//         minimum_balance_for_rent_exemption,
//     ).unwrap();
//     */
// }

// fn execute_instructions(
//     signers: &Vec<&dyn Signer>,
//     client: &RpcClient,
//     fee_payer: &Pubkey,
//     instructions: &[Instruction],
//     minimum_balance_for_rent_exemption: u64,
// ) -> Result<(), Error> {
//     /*let message = if let Some(nonce_account) = config.nonce_account.as_ref() {
//         Message::new_with_nonce(
//             instructions,
//             fee_payer,
//             nonce_account,
//             config.nonce_authority.as_ref().unwrap(),
//         )
//     } else {
//         Message::new(&instructions, fee_payer)
//     };*/

//     let message = Message::new(instructions, Some(fee_payer));

//     let (recent_blockhash, fee_calculator) = client.get_recent_blockhash()?;

//     let balance = client.get_balance(fee_payer)?;

//     if balance < minimum_balance_for_rent_exemption + fee_calculator.calculate_fee(&message) {
//         panic!("insufficient balance");
//     }

//     let mut transaction = Transaction::new_unsigned(message);

//     transaction.try_sign(signers, recent_blockhash)?;

//     let signature = client.send_and_confirm_transaction(&transaction)?;

//     Ok(())
// }

// type CommandResult = Result<(u64, Vec<Instruction>), Error>;
// type Error = Box<dyn std::error::Error>;

// // https://spl.solana.com/associated-token-account
// // https://github.com/solana-labs/solana-program-library/blob/master/token/cli/src/main.rs#L555
// #[allow(clippy::too_many_arguments)]
// fn command_transfer(
//     client: &RpcClient,
//     fee_payer: &Pubkey,
//     token: Pubkey,
//     ui_amount: f64,
//     recipient: Pubkey,
//     sender: Option<Pubkey>,
//     sender_owner: Pubkey,
//     allow_unfunded_recipient: bool,
//     fund_recipient: bool,
//     memo: Option<String>,
// ) -> CommandResult {
//     let sender = if let Some(sender) = sender {
//         sender
//     } else {
//         spl_associated_token_account::get_associated_token_address(&sender_owner, &token)
//     };
//     let (_, decimals) = resolve_mint_info(client, &recipient).unwrap();
//     let transfer_balance = spl_token::ui_amount_to_amount(ui_amount, decimals);
//     let transfer_balance = {
//         let sender_token_amount = client
//             .get_token_account_balance(&sender)
//             .map_err(|err| {
//                 format!(
//                     "Error: Failed to get token balance of sender address {}: {}",
//                     sender, err
//                 )
//             })
//             .unwrap();

//         let sender_balance = sender_token_amount
//             .amount
//             .parse::<u64>()
//             .map_err(|err| {
//                 format!(
//                     "Token account {} balance could not be parsed: {}",
//                     sender, err
//                 )
//             })
//             .unwrap();

//         println!(
//             "Transfer {} tokens\n  Sender: {}\n  Recipient: {}",
//             spl_token::amount_to_ui_amount(transfer_balance, decimals),
//             sender,
//             recipient
//         );

//         if transfer_balance > sender_balance {
//             panic!(
//                 "Error: Sender has insufficient funds, current balance is {}",
//                 sender_token_amount.real_number_string_trimmed()
//             );
//         }

//         transfer_balance
//     };

//     let mut instructions = vec![];

//     let mut recipient_token_account = recipient;
//     let mut minimum_balance_for_rent_exemption = 0;

//     let recipient_is_token_account = {
//         let recipient_account_info = client
//             .get_account_with_commitment(&recipient, client.commitment())?
//             .value
//             .map(|account| {
//                 account.owner == spl_token::id()
//                     && account.data.len() == spl_token::state::Account::LEN
//             });

//         if recipient_account_info.is_none() && !allow_unfunded_recipient {
//             return Err("Error: The recipient address is not funded. \
//                                     Add `--allow-unfunded-recipient` to complete the transfer \
//                                    "
//             .into());
//         }

//         recipient_account_info.unwrap_or(false)
//     };

//     if !recipient_is_token_account {
//         recipient_token_account =
//             spl_associated_token_account::get_associated_token_address(&recipient, &token);
//         println!(
//             "  Recipient associated token account: {}",
//             recipient_token_account
//         );

//         let needs_funding = {
//             if let Some(recipient_token_account_data) = client
//                 .get_account_with_commitment(&recipient_token_account, client.commitment())?
//                 .value
//             {
//                 if recipient_token_account_data.owner == system_program::id() {
//                     true
//                 } else if recipient_token_account_data.owner == spl_token::id() {
//                     false
//                 } else {
//                     return Err(
//                         format!("Error: Unsupported recipient address: {}", recipient).into(),
//                     );
//                 }
//             } else {
//                 true
//             }
//         };

//         if needs_funding {
//             if fund_recipient {
//                 minimum_balance_for_rent_exemption += client
//                     .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)?;
//                 instructions.push(
//                     spl_associated_token_account::create_associated_token_account(
//                         fee_payer, &recipient, &token,
//                     ),
//                 );
//             } else {
//                 return Err(
//                     "Error: Recipient's associated token account does not exist. \
//                                     Add `--fund-recipient` to fund their account"
//                         .into(),
//                 );
//             }
//         }
//     }

//     instructions.push(
//         transfer_checked(
//             &spl_token::id(),
//             &sender,
//             &token,
//             &recipient_token_account,
//             &sender_owner,
//             &[&sender, fee_payer],
//             transfer_balance,
//             decimals,
//         )
//         .unwrap(),
//     );

//     if let Some(text) = memo {
//         instructions.push(spl_memo::build_memo(text.as_bytes(), &[fee_payer]));
//     }

//     Ok((minimum_balance_for_rent_exemption, instructions))
// }

// fn command_create_account(
//     client: &RpcClient,
//     fee_payer: Pubkey,
//     token: Pubkey,
//     owner: Pubkey,
//     maybe_account: Option<Pubkey>,
// ) -> CommandResult {
//     let minimum_balance_for_rent_exemption = client
//         .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)
//         .unwrap();

//     let (account, system_account_ok, instructions) = if let Some(account) = maybe_account {
//         (
//             account,
//             false,
//             vec![
//                 system_instruction::create_account(
//                     &fee_payer,
//                     &account,
//                     minimum_balance_for_rent_exemption,
//                     spl_token::state::Account::LEN as u64,
//                     &spl_token::id(),
//                 ),
//                 spl_token::instruction::initialize_account(
//                     &spl_token::id(),
//                     &account,
//                     &token,
//                     &owner,
//                 )
//                 .unwrap(),
//             ],
//         )
//     } else {
//         let account = spl_associated_token_account::get_associated_token_address(&owner, &token);
//         (
//             account,
//             true,
//             vec![
//                 spl_associated_token_account::create_associated_token_account(
//                     &fee_payer, &owner, &token,
//                 ),
//             ],
//         )
//     };

//     if let Some(account_data) = client
//         .get_account_with_commitment(&account, client.commitment())
//         .unwrap()
//         .value
//     {
//         if !(account_data.owner == system_program::id() && system_account_ok) {
//             panic!("Error: Account already exists: {}", account);
//         }
//     }

//     Ok((minimum_balance_for_rent_exemption, instructions))
// }

// // checks mint account's decimals
// // https://github.com/solana-labs/solana-program-library/blob/707382ee96c1197b50ab3e837b3c46b975e75a4f/token/cli/src/main.rs#L516
// pub(crate) fn resolve_mint_info(
//     client: &RpcClient,
//     token_account: &Pubkey,
// ) -> Result<(Pubkey, u8), Error> {
//     let source_account = client.get_token_account(token_account).unwrap().unwrap();
//     let source_mint = Pubkey::from_str(&source_account.mint).unwrap();
//     Ok((source_mint, source_account.token_amount.decimals))
// }

// fn command_mint(
//     client: &RpcClient,
//     token: Pubkey,
//     ui_amount: f64,
//     recipient: Pubkey,
//     mint_authority: Pubkey,
// ) -> CommandResult {
//     let (_, decimals) = resolve_mint_info(client, &recipient)?;
//     let amount = spl_token::ui_amount_to_amount(ui_amount, decimals);

//     let instructions = vec![mint_to_checked(
//         &spl_token::id(),
//         &token,
//         &recipient,
//         &mint_authority,
//         &[&token, &mint_authority],
//         amount,
//         decimals,
//     )
//     .unwrap()];

//     Ok((0, instructions))
// }

// fn command_create_token(
//     rpc_client: &RpcClient,
//     fee_payer: &Pubkey,
//     decimals: u8,
//     token: &Pubkey,
//     authority: Pubkey,
//     memo: &str,
// ) -> CommandResult {
//     let minimum_balance_for_rent_exemption =
//         rpc_client.get_minimum_balance_for_rent_exemption(Mint::LEN)?;

//     let freeze_authority_pubkey = Some(authority);

//     let instructions = vec![
//         system_instruction::create_account(
//             fee_payer,
//             token,
//             minimum_balance_for_rent_exemption,
//             Mint::LEN as u64,
//             &spl_token::id(),
//         ),
//         spl_token::instruction::initialize_mint(
//             &spl_token::id(),
//             token,
//             &authority,
//             freeze_authority_pubkey.as_ref(),
//             decimals,
//         )?,
//         spl_memo::build_memo(memo.as_bytes(), &[fee_payer]),
//     ];

//     Ok((minimum_balance_for_rent_exemption, instructions))
// }

// // https://docs.solana.com/wallet-guide/paper-wallet#verifying-the-keypair
// // https://github.com/heavy-duty/platform/tree/f91b1db1ff99d0559d4043f556371ff455cb3a14/apps/bulldozer-programs/programs/bulldozer/src
// // https://github.com/heavy-duty/platform/blob/f91b1db1ff99d0559d4043f556371ff455cb3a14/apps/bulldozer-client/src/main.ts

// // TODO
// // https://docs.solana.com/cluster/bench-tps