use std::time::Duration;
use std::{collections::HashMap, sync::Arc};

use super::super::Ctx;
use maplit::hashmap;
use mpl_token_metadata::state::{Collection, Creator, UseMethod, Uses};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::keypair::Keypair, signer::Signer};
use uuid::Uuid;

use arloader::status::OutputFormat;
use arloader::{commands::command_upload_nfts, status::StatusCode};

use sunshine_core::msg::NodeId;

use crate::{Error, Value};

use solana_sdk::signer::keypair::write_keypair_file;

use arloader::Arweave;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArweaveUpload {
    pub fee_payer: Option<NodeId>,
    pub reward_mult: Option<f32>,
    pub file_path: Option<String>,
}

impl ArweaveUpload {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let fee_payer = match self.fee_payer {
            Some(s) => ctx.get_keypair_by_id(s).await?,
            None => match inputs.remove("fee_payer") {
                Some(Value::NodeId(s)) => ctx.get_keypair_by_id(s).await?,
                Some(Value::Keypair(k)) => k.into(),
                _ => return Err(Error::ArgumentNotFound("fee_payer".to_string())),
            },
        };

        let reward_mult = match self.reward_mult {
            Some(s) => s,
            None => match inputs.remove("reward_mult") {
                Some(Value::F32(s)) => s,
                _ => return Err(Error::ArgumentNotFound("reward_mult".to_string())),
            },
        };

        let file_path = match &self.file_path {
            Some(s) => s.clone(),
            None => match inputs.remove("file_path") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("file_path".to_string())),
            },
        };

        /*
        let arweave = Arweave::default();

        let price_terms = arweave.get_price_terms(reward_mult).await?;

        let mut status = arweave
            .upload_file_from_path_with_sol(
                file_path.into(),
                None,
                None,
                None,
                price_terms,
                ctx.solana_url.clone(),
                ctx.solana_arweave_url.clone(),
                &fee_payer,
            )
            .await?;

        loop {
            match status.status {
                StatusCode::Confirmed => break,
                StatusCode::NotFound => return Err(Error::ArweaveTxNotFound),
                StatusCode::Submitted | StatusCode::Pending => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    status = arweave.get_status(&status.id).await?;
                }
            }
        }
        */

        let outputs = hashmap! {
            "fee_payer".to_owned() => Value::Keypair(fee_payer.into()),
            //"file_uri".to_owned() => Value::String(format!("https://arweave.net/{}", status.id.to_string())),
        };

        Ok(outputs)
    }
}

/*


/// Gets cost of uploading a list of files.
pub async fn command_get_cost<IP>(
    arweave: &Arweave,
    paths_iter: IP,
    reward_mult: f32,
    with_sol: bool,
    bundle_size: u64,
    no_bundle: bool,
) -> CommandResult
where
    IP: Iterator<Item = PathBuf> + Send + Sync,
{
    let (base, incremental) = arweave.get_price_terms(reward_mult).await?;
    let (_, usd_per_ar, usd_per_sol) = arweave.get_price(&1).await?;

    let units = match with_sol {
        true => "lamports",
        false => "winstons",
    };

    let (num_trans, num_files, cost, bytes) = if no_bundle {
        paths_iter.fold((0, 0, 0, 0), |(n_t, n_f, c, b), p| {
            let data_len = p.metadata().unwrap().len();
            (
                n_t + 1,
                n_f + 1,
                c + {
                    let blocks_len = data_len / BLOCK_SIZE + (data_len % BLOCK_SIZE != 0) as u64;
                    match with_sol {
                        true => {
                            std::cmp::max((base + incremental * (blocks_len - 1)) / RATE, FLOOR)
                                + 5000
                        }
                        false => base + incremental * (blocks_len - 1),
                    }
                },
                b + data_len,
            )
        })
    } else {
        let path_chunks = arweave.chunk_file_paths(paths_iter, bundle_size)?;
        path_chunks.iter().fold(
            (0, 0, 0, 0),
            |(n_t, n_f, c, b), PathsChunk(paths, data_len)| {
                (
                    n_t + 1,
                    n_f + paths.len(),
                    c + {
                        let blocks_len =
                            data_len / BLOCK_SIZE + (data_len % BLOCK_SIZE != 0) as u64;
                        match with_sol {
                            true => {
                                std::cmp::max((base + incremental * (blocks_len - 1)) / RATE, FLOOR)
                                    + 5000
                            }
                            false => base + incremental * (blocks_len - 1),
                        }
                    },
                    b + data_len,
                )
            },
        )
    };

    // get usd cost based on calculated cost
    let usd_cost = match with_sol {
        true => (&cost * &usd_per_sol).to_f32().unwrap() / 1e11_f32,
        false => (&cost * &usd_per_ar).to_f32().unwrap() / 1e14_f32,
    };

    println!(
        "The price to upload {} files with {} total bytes in {} transaction(s) is {} {} (${:.4}).",
        num_files, bytes, num_trans, cost, units, usd_cost
    );

    Ok(())
}



/// Uploads files to Arweave.
pub async fn command_upload<IP>(
    arweave: &Arweave,
    paths_iter: IP,
    log_dir: Option<PathBuf>,
    tags: Option<Vec<Tag<Base64>>>,
    reward_mult: f32,
    output_format: &OutputFormat,
    buffer: usize,
) -> CommandResult
where
    IP: Iterator<Item = PathBuf> + Send + Sync,
{
    let price_terms = arweave.get_price_terms(reward_mult).await?;

    let mut stream = upload_files_stream(
        arweave,
        paths_iter,
        tags,
        log_dir.clone(),
        None,
        price_terms,
        buffer,
    );

    let mut counter = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(status) => {
                if counter == 0 {
                    if let Some(log_dir) = &log_dir {
                        println!("Logging statuses to {}", &log_dir.display());
                    }
                    println!("{}", status.header_string(&output_format));
                }
                print!("{}", output_format.formatted_string(&status));
                counter += 1;
            }
            Err(e) => println!("{:#?}", e),
        }
    }

    if counter == 0 {
        println!("<FILE_PATHS> didn't match any files.");
    } else {
        println!(
            "Uploaded {} files. Run `arloader update-status {} --file-paths <FILE_PATHS>` to confirm transaction(s).",
            counter,
            &log_dir.unwrap_or(PathBuf::from("")).display(),
        );
    }

    Ok(())
}




/// Uploads files matching glob pattern, returning a stream of [`Status`] structs, paying with SOL.
pub fn upload_files_with_sol_stream<'a, IP>(
    arweave: &'a Arweave,
    paths_iter: IP,
    tags: Option<Vec<Tag<Base64>>>,
    log_dir: Option<PathBuf>,
    last_tx: Option<Base64>,
    price_terms: (u64, u64),
    solana_url: Url,
    sol_ar_url: Url,
    from_keypair: &'a Keypair,
    buffer: usize,
) -> impl Stream<Item = Result<Status, Error>> + 'a
where
    IP: Iterator<Item = PathBuf> + Send + Sync + 'a,
{
    stream::iter(paths_iter)
        .map(move |p| {
            arweave.upload_file_from_path_with_sol(
                p,
                log_dir.clone(),
                tags.clone(),
                last_tx.clone(),
                price_terms,
                solana_url.clone(),
                sol_ar_url.clone(),
                from_keypair,
            )
        })
        .buffer_unordered(buffer)
}






    println!("<FILE_PATHS> didn't match any files.");
    } else {
        println!(
            "Uploaded {} files. Run `arloader update-status {} --file-paths <FILE_PATHS>` to confirm transaction(s).",
            counter,
            &log_dir.unwrap_or(PathBuf::from("")).display(),
        );
    }


/// Gets status from the network for the provided transaction id.
pub async fn command_get_status(
    arweave: &Arweave,
    id: &str,
    output_format: &OutputFormat,
) -> CommandResult {
    let id = Base64::from_str(id)?;
    let status = arweave.get_status(&id).await?;
    println!(
        "{}",
        status
            .header_string(output_format)
            .split_at(32)
            .1
            .split_at(132)
            .0
    );
    print!("{}", output_format.formatted_string(&status).split_at(32).1);
    Ok(())
}
*/
