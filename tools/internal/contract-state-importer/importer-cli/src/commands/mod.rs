// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::commands::initialise_with_state::{initialise_with_state, InitialiseWithStateArgs};
use crate::commands::prepare::{execute_prepare_contract, PrepareArgs};
use crate::commands::set_state::{execute_set_state, SetStateArgs};
use crate::commands::swap_contract::{execute_swap_contract, SwapContractArgs};
use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use std::path::PathBuf;
use std::sync::OnceLock;

pub mod initialise_mixnet_vesting_with_states;
pub mod initialise_with_state;
pub mod prepare;
pub mod set_state;
pub mod swap_contract;

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub struct Cli {
    /// Path pointing to an env file that configures the CLI.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<PathBuf>,

    #[clap(long)]
    pub(crate) mnemonic: bip39::Mnemonic,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Upload and instantiates the importer contract
    PrepareContract(PrepareArgs),

    /// Set the state of the previously instantiated importer contract with the provided state dump
    SetState(SetStateArgs),

    /// Swap the importer contract code with the one corresponding to the previously uploaded state dump
    SwapContract(SwapContractArgs),

    /// Combines the functionalities of `prepare-contract`, `set-state` and `swap-contract`
    InitialiseWithState(InitialiseWithStateArgs),
}

impl Cli {
    pub async fn execute(self) -> anyhow::Result<()> {
        let network_details = NymNetworkDetails::new_from_env();
        let client_config = nyxd::Config::try_from_nym_network_details(&network_details)?;
        let nyxd_url = network_details
            .endpoints
            .first()
            .expect("network details are not defined")
            .nyxd_url
            .as_str();

        let client = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            client_config,
            nyxd_url,
            self.mnemonic,
        )?;
        match self.command {
            Commands::PrepareContract(args) => execute_prepare_contract(args, client).await,
            Commands::SetState(args) => execute_set_state(args, client).await,
            Commands::SwapContract(args) => execute_swap_contract(args, client).await,
            Commands::InitialiseWithState(args) => initialise_with_state(args, client).await,
        }
    }
}
