// use self::{
//     account::CreateAccount,
//     keypair::GenerateKeypair,
//     token::{CreateToken, MintToken},
//     transfer::Transfer,
// };

pub mod account;
pub mod keypair;
pub mod token;
pub mod transfer;

// use super::Msg;
struct Ctx {
    client: RpcClient,
    keyring: DashMap<String, Arc<Keypair>>,
    pub_keys: DashMap<String, Pubkey>,
}

#[derive(Debug)]
struct Config {
    url: String,
    keyring: HashMap<String, GenerateKeypair>,
    pub_keys: HashMap<String, String>,
    db: Arc<dyn Datastore>,
}

impl Ctx {
    fn new(cfg: Config) -> Result<Ctx, Error> {
        let keyring = cfg
            .keyring
            .into_iter()
            .map(|(name, gen_keypair)| {
                let keypair = generate_keypair(&gen_keypair.passphrase, &gen_keypair.seed_phrase)?;

                println!("pubkey: {}", keypair.pubkey());

                Ok((name, Arc::new(keypair)))
            })
            .collect::<Result<DashMap<_, _>, Error>>()?;

        let pub_keys = cfg
            .pub_keys
            .into_iter()
            .map(|(name, pubkey)| Ok((name, Pubkey::from_str(&pubkey)?)))
            .chain(
                keyring
                    .iter()
                    .map(|kp| Ok((kp.key().clone(), kp.value().pubkey()))),
            )
            .collect::<Result<DashMap<_, _>, Error>>()?;
        
        Ok(Ctx 
            client: RpcClient::new(cfg.url),
            keyring,
            pub_keys,
        })
    }

    fn get_keypair(&self, name: &str) -> Result<Arc<Keypair>, Error> {
        self.keyring
            .get(name)
            .map(|r| r.value().clone())
            .ok_or(Box::new(CustomError::KeypairDoesntExist))
    }

    fn get_pubkey(&self, name: &str) -> Result<Pubkey, Error> {
        self.pub_keys
            .get(name)
            .map(|pk| *pk)
            .ok_or(Box::new(CustomError::PubkeyDoesntExist))
    }
}

pub struct Command {
    ctx: Arc<Mutex<Ctx>>,
    kind: Kind,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Kind {
    GenerateKeypair(Option<keypair::GenerateConfig>),
    DeleteKeypair(Option<keypair::DeleteConfig>),
    AddPubkey(Option<keypair::AddPubConfig>),
    DeletePubkey(String),
    CreateAccount(CreateAccount),
    GetBalance(String),
    CreateToken(CreateToken),
    MintToken(MintToken),
    RequestAirdrop(String, u64),
    Transfer(Transfer),
}

impl Command {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, Msg>,
    ) -> Result<HashMap<String, Msg>, Error> {
        match self {
            /*
        Command::GenerateKeypair(name, gen_keypair) => {
            if exec_ctx.keyring.contains_key(name) {
                return Err(Box::new(CustomError::KeypairAlreadyExistsInKeyring));
            }

            if exec_ctx.pub_keys.contains_key(name) {
                return Err(Box::new(CustomError::PubkeyAlreadyExists));
            }

            let keypair = generate_keypair(&gen_keypair.passphrase, &gen_keypair.seed_phrase)?;
            // let keypair = Arc::new(keypair);

            exec_ctx.pub_keys.insert(name.clone(), keypair.pubkey());
            exec_ctx.keyring.insert(name.clone(), Arc::new(keypair));

            Ok(CommandResponse::Success)
        }
        Command::DeleteKeypair(name) => {
            if exec_ctx.keyring.remove(name).is_none() {
                return Err(Box::new(CustomError::KeypairDoesntExist));
            }maplit
        Command::AddPubkey(name, pubkey) => {
            if exec_ctx.pub_keys.contains_key(name) {
                return Err(Box::new(CustomError::PubkeyAlreadyExists));
            }

            let pubkey = Pubkey::from_str(pubkey)?;

            exec_ctx.pub_keys.insert(name.clone(), pubkey);

            Ok(CommandResponse::Success)
        }
        Command::DeletePubkey(name) => {
            if exec_ctx.pub_keys.remove(name).is_none() {
                return Err(Box::new(CustomError::PubkeyDoesntExist));
            }

            Ok(CommandResponse::Success)
        }
        Command::CreateAccount(create_account) => {
            let owner = exec_ctx.get_pubkey(&create_account.owner)?;
            let fee_payer = exec_ctx.get_keypair(&create_account.fee_payer)?;
            let token = exec_ctx.get_pubkey(&create_account.token)?;
            let account = match create_account.account {
                Some(ref account) => Some(exec_ctx.get_keypair(account)?),
                None => None,
            };

            let (minimum_balance_for_rent_exemption, instructions) = command_create_account(
                &exec_ctx.client,
                fee_payer.pubkey(),
                token,
                owner,
                account.as_ref().map(|a| a.pubkey()),
            )
            .unwrap();

            let mut signers: Vec<Arc<dyn Signer>> = vec![fee_payer.clone()];

            if let Some(account) = account {
                signers.push(account.clone());
            };

            execute_instructions(
                &signers,
                &exec_ctx.client,
                &fee_payer.pubkey(),
                &instructions,
                minimum_balance_for_rent_exemption,
            )?;

            Ok(CommandResponse::Success)
        }
        Command::GetBalance(name) => {
            let pubkey = exec_ctx.get_pubkey(&name)?;

            let balance = exec_ctx.client.get_balance(&pubkey)?;

            Ok(CommandResponse::Balance(balance))
        }
        Command::CreateToken(create_token) => {
            let fee_payer = exec_ctx.get_keypair(&create_token.fee_payer)?;
            let authority = exec_ctx.get_keypair(&create_token.authority)?;
            let token = exec_ctx.get_keypair(&create_token.token)?;

            let (minimum_balance_for_rent_exemption, instructions) = command_create_token(
                &exec_ctx.client,
                &fee_payer.pubkey(),
                create_token.decimals,
                &token.pubkey(),
                authority.pubkey(),
                &create_token.memo,
            )?;

            let signers: Vec<Arc<dyn Signer>> =
                vec![authority.clone(), fee_payer.clone(), token.clone()];

            execute_instructions(
                &signers,
                &exec_ctx.client,
                &fee_payer.pubkey(),
                &instructions,
                minimum_balance_for_rent_exemption,
            )?;

            Ok(CommandResponse::Success)
        }
        Command::RequestAirdrop(name, amount) => {
            let pubkey = exec_ctx.get_pubkey(&name)?;

            exec_ctx.client.request_airdrop(&pubkey, *amount)?;

            Ok(CommandResponse::Success)
        }
        Command::MintToken(mint_token) => {
            let token = exec_ctx.get_keypair(&mint_token.token)?;
            let mint_authority = exec_ctx.get_keypair(&mint_token.mint_authority)?;
            let recipient = exec_ctx.get_pubkey(&mint_token.recipient)?;
            let fee_payer = exec_ctx.get_keypair(&mint_token.fee_payer)?;

            let (minimum_balance_for_rent_exemption, instructions) = command_mint(
                &exec_ctx.client,
                token.pubkey(),
                mint_token.amount,
                recipient,
                mint_authority.pubkey(),
            )?;

            let signers: Vec<Arc<dyn Signer>> =
                vec![mint_authority.clone(), token.clone(), fee_payer.clone()];

            execute_instructions(
                &signers,
                &exec_ctx.client,
                &fee_payer.pubkey(),
                &instructions,
                minimum_balance_for_rent_exemption,
            )?;

            Ok(CommandResponse::Success)
        }
        Command::Transfer(transfer) => {
            let token = exec_ctx.get_pubkey(&transfer.token)?;
            let recipient = exec_ctx.get_pubkey(&transfer.recipient)?;
            let fee_payer = exec_ctx.get_keypair(&transfer.fee_payer)?;
            let sender = match transfer.sender {
                Some(ref sender) => Some(exec_ctx.get_keypair(sender)?),
                None => None,
            };
            let sender_owner = exec_ctx.get_keypair(&transfer.sender_owner)?;

            let (minimum_balance_for_rent_exemption, instructions) = command_transfer(
                &exec_ctx.client,
                &fee_payer.pubkey(),
                token,
                transfer.amount,
                recipient,
                sender.as_ref().map(|s| s.pubkey()),
                sender_owner.pubkey(),
                transfer.allow_unfunded_recipient,
                transfer.fund_recipient,
                transfer.memo.clone(),
            )?;

            let mut signers: Vec<Arc<dyn Signer>> =
                vec![fee_payer.clone(), sender_owner.clone()];

            if let Some(sender) = sender {
                signers.push(sender);
            }

            execute_instructions(
                &signers,
                &exec_ctx.client,
                &fee_payer.pubkey(),
                &instructions,
                minimum_balance_for_rent_exemption,
            )?;

            Ok(CommandResponse::Success)
        }
        Command::Print(s) => {
            println!("{}", s);
            Ok(CommandResponse::Success)
        }
        */
        }
    }
}