// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod client;
mod commands;
mod error;
mod state;

use commands::*;
use completions::fig_generate;
use config::{DATA_DIR, DB_FILE_NAME};
use error::Result;
use network_defaults::{setup_env, NymNetworkDetails};
use std::time::Duration;

use clap::{CommandFactory, Parser};
use validator_client::nyxd::traits::DkgQueryClient;
use validator_client::nyxd::CosmWasmClient;
use validator_client::Config;

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
        if epoch.state.is_final() {
            break;
        } else {
            let secs_until_final = epoch.secs_until_final();
            println!("Approximately {} seconds until coconut is available. Sleeping until then. You can safely kill the process at any moment.", secs_until_final);
            std::thread::sleep(Duration::from_secs(secs_until_final));
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    setup_env(args.config_env_file.as_ref());
    let bin_name = "nym-credential-client";

    match args.command {
        Command::Run(r) => {
            let db_path = r.client_home_directory.join(DATA_DIR).join(DB_FILE_NAME);
            let shared_storage = credential_storage::initialise_storage(db_path).await;

            let network_details = NymNetworkDetails::new_from_env();
            let config = Config::try_from_nym_network_details(&network_details)?;
            let client = validator_client::Client::new_query(config)?;

            block_until_coconut_is_available(&client).await?;
            println!("Finished sleeping, starting depositing funds, don't kill the process");

            let state = deposit(&r.nyxd_url, &r.mnemonic, r.amount).await?;

            get_credential(&state, client, shared_storage).await?;
        }
        Command::Completions(c) => c.generate(&mut crate::Cli::command(), bin_name),
        Command::GenerateFigSpec => fig_generate(&mut crate::Cli::command(), bin_name),
    }

    Ok(())
}
