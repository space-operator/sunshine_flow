use std::sync::Arc;

use solana_client::rpc_client::RpcClient;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;

use crate::error::Error;

pub(crate) fn execute(
    signers: &[Arc<dyn Signer>],
    client: &RpcClient,
    fee_payer: &Pubkey,
    instructions: &[Instruction],
    minimum_balance_for_rent_exemption: u64,
) -> Result<Signature, Error> {
    /*let message = if let Some(nonce_account) = config.nonce_account.as_ref() {
        Message::new_with_nonce(
            instructions,
            fee_payer,
            nonce_account,
            config.nonce_authority.as_ref().unwrap(),
        )
    } else {
        Message::new(&instructions, fee_payer)
    };*/

    let message = Message::new(instructions, Some(fee_payer));

    let (recent_blockhash, fee_calculator) = client.get_recent_blockhash()?;

    let balance = client.get_balance(fee_payer)?;

    if balance < minimum_balance_for_rent_exemption + fee_calculator.calculate_fee(&message) {
        panic!("insufficient balance");
    }

    let mut transaction = Transaction::new_unsigned(message);

    let signers = signers
        .iter()
        .map(|s| s.as_ref())
        .collect::<Vec<&dyn Signer>>();

    transaction.try_sign(&signers, recent_blockhash)?;

    let signature = client.send_and_confirm_transaction(&transaction)?;

    Ok(signature)
}
