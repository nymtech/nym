// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{CommandFactory, Parser, Subcommand};
use log::{error, warn};
use nym_bin_common::logging::setup_logging;
use nym_cli_commands::context::{get_network_details, ClientArgs};
use nym_validator_client::nyxd::AccountId;

mod coconut;
mod completion;
mod validator;

#[derive(Debug, Parser)]
#[clap(name = "nym-cli")]
#[clap(about = "A client for interacting with Nym smart contracts and the Nyx blockchain", long_about = None)]
pub(crate) struct Cli {
    #[clap(long, global = true)]
    #[clap(
        help = "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC."
    )]
    pub(crate) mnemonic: Option<bip39::Mnemonic>,

    #[clap(short, long, global = true)]
    #[clap(
        help = "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file."
    )]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[clap(long, global = true)]
    #[clap(
        help = "Overrides the nyxd URL provided either as an environment variable NYXD_VALIDATOR or in a config file"
    )]
    pub(crate) nyxd_url: Option<String>,

    #[clap(long, global = true)]
    #[clap(
        help = "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file"
    )]
    pub(crate) nym_api_url: Option<String>,

    #[clap(long, global = true)]
    #[clap(
        help = "Overrides the mixnet contract address provided either as an environment variable or in a config file"
    )]
    pub(crate) mixnet_contract_address: Option<AccountId>,

    #[clap(long, global = true)]
    #[clap(
        help = "Overrides the vesting contract address provided either as an environment variable or in a config file"
    )]
    pub(crate) vesting_contract_address: Option<AccountId>,

    #[clap(subcommand)]
    command: Commands,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    /// Query and manage Nyx blockchain accounts
    Account(nym_cli_commands::validator::account::Account),
    /// Sign and verify messages
    Signature(nym_cli_commands::validator::signature::Signature),
    /// Ecash related stuff
    Ecash(nym_cli_commands::ecash::Ecash),
    /// Query chain blocks
    Block(nym_cli_commands::validator::block::Block),
    /// Manage and execute WASM smart contracts
    Cosmwasm(nym_cli_commands::validator::cosmwasm::Cosmwasm),
    /// Query for transactions
    Tx(nym_cli_commands::validator::transactions::Transactions),
    /// Create and query for a vesting schedule
    VestingSchedule(nym_cli_commands::validator::vesting::VestingSchedule),
    /// Manage your mixnet infrastructure, delegate stake or query the directory
    Mixnet(nym_cli_commands::validator::mixnet::Mixnet),
    /// Generates shell completion
    GenerateFig,
}

async fn execute(cli: Cli) -> anyhow::Result<()> {
    let args = ClientArgs {
        nyxd_url: cli.nyxd_url,
        nym_api_url: cli.nym_api_url,
        mnemonic: cli.mnemonic,
        mixnet_contract_address: cli.mixnet_contract_address,
        vesting_contract_address: cli.vesting_contract_address,
        config_env_file: cli.config_env_file,
    };

    let network_details = get_network_details(&args)?;

    // use the --mnemonic option if set, then try fall back to the MNEMONIC env var
    let mnemonic = args.mnemonic.clone().or_else(|| {
        std::env::var("MNEMONIC")
            .ok()
            .and_then(|m| bip39::Mnemonic::parse(m).ok())
    });

    match cli.command {
        Commands::Account(account) => {
            validator::account::execute(args, account, &network_details, mnemonic).await?
        }
        Commands::Signature(signature) => {
            validator::signature::execute(signature, &network_details, mnemonic).await?
        }
        Commands::Ecash(coconut) => coconut::execute(args, coconut, &network_details).await?,
        Commands::Block(block) => validator::block::execute(block, &network_details).await?,
        Commands::Cosmwasm(cosmwasm) => {
            validator::cosmwasm::execute(args, cosmwasm, &network_details).await?
        }
        Commands::Tx(transactions) => {
            validator::transactions::execute(transactions, &network_details).await?
        }
        Commands::VestingSchedule(vesting) => {
            validator::vesting::execute(args, vesting, &network_details).await?
        }
        Commands::Mixnet(mixnet) => {
            validator::mixnet::execute(args, mixnet, &network_details).await?
        }
        Commands::GenerateFig => {
            let mut cmd = Cli::command();
            completion::print_fig(&mut cmd);
        }
    }

    Ok(())
}

async fn wait_for_interrupt() {
    if let Err(e) = tokio::signal::ctrl_c().await {
        error!(
            "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
            e
        );
    }
    println!(
        "Received SIGINT - the process will terminate now (threads are not yet nicely stopped, if you see stack traces that's alright)."
    );
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logging();

    let cli = Cli::parse();

    tokio::select! {
        _ = wait_for_interrupt() => {
            warn!("Received interrupt - the specified command might have not completed!");
            Ok(())
        },
        res = execute(cli) => res
    }
}
