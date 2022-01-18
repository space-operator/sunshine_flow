use solana_client::rpc_client::RpcClient;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;

use crate::error::Error;

#[allow(clippy::ptr_arg)]
pub(crate) fn execute(
    signers: &Vec<&dyn Signer>,
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

    let recent_blockhash = client.get_latest_blockhash()?;

    let message = Message::new_with_blockhash(instructions, Some(fee_payer), &recent_blockhash);

    let balance = client.get_balance(fee_payer)?;

    let needed = minimum_balance_for_rent_exemption + client.get_fee_for_message(&message)?;

    if balance < needed {
        panic!("insufficient balance: have={}; needed={};", balance, needed);
    }

    let mut transaction = Transaction::new_unsigned(message);

    transaction.try_sign(signers, recent_blockhash)?;

    let signature = client.send_and_confirm_transaction(&transaction)?;

    Ok(signature)
}

// #[allow(deprecated)]
// pub fn get_fee_for_message(&self, message: &Message) -> ClientResult<u64> {
//     if self.get_node_version()? < semver::Version::new(1, 9, 0) {
//         let fee_calculator = self
//             .get_fee_calculator_for_blockhash(&message.recent_blockhash)?
//             .ok_or_else(|| ClientErrorKind::Custom("Invalid blockhash".to_string()))?;
//         Ok(fee_calculator
//             .lamports_per_signature
//             .saturating_mul(message.header.num_required_signatures as u64))
//     } else {
//         let serialized_encoded =
//             serialize_and_encode::<Message>(message, UiTransactionEncoding::Base64)?;
//         let result = self.send::<Response<Option<u64>>>(
//             RpcRequest::GetFeeForMessage,
//             json!([serialized_encoded, self.commitment()]),
//         )?;
//         result
//             .value
//             .ok_or_else(|| ClientErrorKind::Custom("Invalid blockhash".to_string()).into())
//     }
// }

// pub fn get_new_latest_blockhash(&self, blockhash: &Hash) -> ClientResult<Hash> {
//     let mut num_retries = 0;
//     let start = Instant::now();
//     while start.elapsed().as_secs() < 5 {
//         if let Ok(new_blockhash) = self.get_latest_blockhash() {
//             if new_blockhash != *blockhash {
//                 return Ok(new_blockhash);
//             }
//         }
//         debug!("Got same blockhash ({:?}), will retry...", blockhash);

//         // Retry ~twice during a slot
//         sleep(Duration::from_millis(DEFAULT_MS_PER_SLOT / 2));
//         num_retries += 1;
//     }
//     Err(RpcError::ForUser(format!(
//         "Unable to get new blockhash after {}ms (retried {} times), stuck at {}",
//         start.elapsed().as_millis(),
//         num_retries,
//         blockhash
//     ))
//     .into())
// }

// pub fn get_transport_stats(&self) -> RpcTransportStats {
//     self.sender.get_transport_stats()
// }
