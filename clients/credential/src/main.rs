// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{CommandFactory, Parser};
use commands::*;
use log::*;
use nym_bin_common::completions::fig_generate;
use nym_bin_common::logging::setup_logging;
use nym_client_core::config::disk_persistence::CommonClientPaths;
use nym_config::DEFAULT_DATA_DIR;
use nym_credential_utils::utils::{block_until_coconut_is_available, recover_credentials};
use nym_credential_utils::{recovery_storage, Result};
use nym_network_defaults::{setup_env, NymNetworkDetails};
use nym_validator_client::nyxd::{Coin, Config};
use nym_validator_client::DirectSigningHttpRpcNyxdClient;

mod commands;

#[derive(Parser)]
#[clap(author = "Nymtech", version, about)]
struct Cli {
    /// Path pointing to an env file that configures the client.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[clap(subcommand)]
    pub(crate) command: Command,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    setup_logging();
    setup_env(args.config_env_file.as_ref());
    let bin_name = "nym-credential-client";

    match args.command {
        Command::Run(r) => {
            // we assume the structure of <home-dir>/data
            let data_dir = r.client_home_directory.join(DEFAULT_DATA_DIR);
            let paths = CommonClientPaths::new_default(data_dir);
            let db_path = paths.credentials_database;

            let shared_storage =
                nym_credential_storage::initialise_persistent_storage(db_path).await;
            let recovery_storage = recovery_storage::RecoveryStorage::new(r.recovery_dir)?;

            let network_details = NymNetworkDetails::new_from_env();
            let config = Config::try_from_nym_network_details(&network_details).expect(
                "failed to construct valid validator client config with the provided network",
            );
            let amount = Coin::new(
                r.amount as u128,
                network_details.chain_details.mix_denom.base,
            );
            let endpoint = network_details.endpoints[0].nyxd_url.as_str();
            let client = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
                config,
                endpoint,
                r.mnemonic.parse().unwrap(),
            )?;

            block_until_coconut_is_available(&client).await?;
            info!("Starting depositing funds, don't kill the process");

            if !r.recovery_mode {
                let state = nym_bandwidth_controller::acquire::deposit(&client, amount).await?;
                if nym_bandwidth_controller::acquire::get_credential(
                    &state,
                    &client,
                    &shared_storage,
                )
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
                recover_credentials(&client, &recovery_storage, &shared_storage).await?;
            }
        }
        Command::Completions(c) => c.generate(&mut Cli::command(), bin_name),
        Command::GenerateFigSpec => fig_generate(&mut Cli::command(), bin_name),
    }

    Ok(())
}
