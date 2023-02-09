// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod client;
mod commands;
mod error;
mod recovery_storage;
mod state;

use commands::*;
use completions::fig_generate;
use config::{DATA_DIR, DB_FILE_NAME};
use error::Result;
use log::*;
use network_defaults::{setup_env, NymNetworkDetails};
use std::process::exit;
use std::time::{Duration, SystemTime};

use clap::{CommandFactory, Parser};
use logging::setup_logging;
use validator_client::nyxd::traits::DkgQueryClient;
use validator_client::nyxd::CosmWasmClient;
use validator_client::Config;

const SAFETY_BUFFER_SECS: u64 = 60; // 1 minute

#[derive(Parser)]
#[clap(author = "Nymtech", version, about)]
struct Cli {
    /// Path pointing to an env file that configures the client.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[clap(subcommand)]
    pub(crate) command: Command,
}

async fn block_until_coconut_is_available<C: Clone + CosmWasmClient + Send + Sync>(
    client: &validator_client::Client<C>,
) -> Result<()> {
    loop {
        let epoch = client.nyxd.get_current_epoch().await?;
        let current_timestamp_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();
        if epoch.state.is_final() {
            if current_timestamp_secs + SAFETY_BUFFER_SECS >= epoch.finish_timestamp.seconds() {
                info!("In the next {} minute(s), a transition will take place in the coconut system. Deposits should be halted in this time for safety reasons.", SAFETY_BUFFER_SECS / 60);
                exit(0);
            }

            break;
        } else {
            // Use 10 additional seconds to avoid the exact moment of going into the final epoch state
            let secs_until_final = epoch.final_timestamp_secs() + 10 - current_timestamp_secs;
            info!("Approximately {} seconds until coconut is available. Sleeping until then. You can safely kill the process at any moment.", secs_until_final);
            std::thread::sleep(Duration::from_secs(secs_until_final));
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    setup_logging();
    setup_env(args.config_env_file.as_ref());
    let bin_name = "nym-credential-client";

    match args.command {
        Command::Run(r) => {
            let db_path = r.client_home_directory.join(DATA_DIR).join(DB_FILE_NAME);
            let shared_storage = credential_storage::initialise_storage(db_path).await;
            let recovery_storage = recovery_storage::RecoveryStorage::new(r.recovery_dir)?;

            let network_details = NymNetworkDetails::new_from_env();
            let config = Config::try_from_nym_network_details(&network_details)?;
            let client = validator_client::Client::new_query(config)?;

            block_until_coconut_is_available(&client).await?;
            info!("Starting depositing funds, don't kill the process");

            if !r.recovery_mode {
                let state = deposit(&r.nyxd_url, &r.mnemonic, r.amount).await?;
                if get_credential(&state, client, shared_storage)
                    .await
                    .is_err()
                {
                    warn!("Failed to obtain credential. Dumping recovery data.",);
                    match recovery_storage.insert_voucher(&state.voucher) {
                        Ok(file_path) => {
                            warn!("Dumped recovery data to {:?}. Try using recovery mode to convert it to a credential", file_path);
                        }
                        Err(e) => {
                            error!("Could not dump recovery data to file system due to {:?}, the deposit will be lost!", e)
                        }
                    }
                }
            } else {
                recover_credentials(client, &recovery_storage, shared_storage).await?;
            }
        }
        Command::Completions(c) => c.generate(&mut crate::Cli::command(), bin_name),
        Command::GenerateFigSpec => fig_generate(&mut crate::Cli::command(), bin_name),
    }

    Ok(())
}
