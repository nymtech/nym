// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use std::path::PathBuf;
use std::sync::OnceLock;

pub(crate) mod build_info;
pub(crate) mod check_prerequisites;
pub(crate) mod down;
pub(crate) mod initialise_contracts;
pub(crate) mod initialise_nym_api;
pub(crate) mod initialise_nym_nodes;
pub(crate) mod initialise_nyxd;
pub(crate) mod purge;
pub(crate) mod rebuild_binaries_image;
pub(crate) mod run_gateway_probe_test;
pub(crate) mod up;

#[derive(clap::Args, Debug)]
pub(crate) struct CommonArgs {
    #[clap(long, group = "storage")]
    pub(crate) localnet_storage_path: Option<PathBuf>,

    #[clap(long)]
    pub(crate) orchestrator_db: Option<PathBuf>,

    #[clap(long)]
    pub(crate) existing_network: Option<String>,

    /// Custom DNS flag ('--dns') to pass to all spawned containers
    #[clap(long)]
    pub(crate) custom_dns: Option<String>,

    /// Specify whether all the data should be cleaned-up after use
    #[clap(long, group = "storage")]
    pub(crate) ephemeral: bool,
}

impl CommonArgs {
    //
}

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    pub(crate) async fn execute(self) -> anyhow::Result<()> {
        match self.command {
            Commands::BuildInfo(args) => build_info::execute(args),
            Commands::InitialiseNyxd(args) => initialise_nyxd::execute(args).await,
            Commands::InitialiseContracts(args) => initialise_contracts::execute(args).await,
            Commands::InitialiseNymApi(args) => initialise_nym_api::execute(args).await,
            Commands::InitialiseNymNodes(args) => initialise_nym_nodes::execute(args).await,
            Commands::RunGatewayProbeTest(args) => run_gateway_probe_test::execute(args).await,
            Commands::RebuildBinariesImage(args) => rebuild_binaries_image::execute(args).await,
            Commands::CheckPrerequisites(args) => check_prerequisites::execute(args).await,
            Commands::Up(args) => up::execute(args).await,
            Commands::Down(args) => down::execute(args).await,
            Commands::Purge(args) => purge::execute(args).await,
        }
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Show build information of this binary
    BuildInfo(build_info::Args),

    /// Initialise new nyxd instance
    InitialiseNyxd(initialise_nyxd::Args),

    /// Upload and initialise all Nym cosmwasm contracts
    InitialiseContracts(initialise_contracts::Args),

    /// Initialise instance of nym api and adjust the DKG contract
    /// to allow it to immediately start issuing zk-nyms
    InitialiseNymApi(initialise_nym_api::Args),

    /// Initialise nym nodes to start serving mixnet (and wireguard) traffic.
    /// this involves bonding them in the contract and starting the containers
    InitialiseNymNodes(initialise_nym_nodes::Args),

    /// Run a gateway probe against the running localnet
    RunGatewayProbeTest(run_gateway_probe_test::Args),

    /// Rebuild the docker and container image used for running the nym binaries
    RebuildBinariesImage(rebuild_binaries_image::Args),

    /// Performs basic prerequisites check for running the orchestrator
    CheckPrerequisites(check_prerequisites::Args),

    /// Single command to start up localnet with minimal configuration
    Up(up::Args),

    /// Stop the localnet (stops and removes all containers using `localnet-*` image
    Down(down::Args),

    /// Remove all localnet information, including any containers and images
    Purge(purge::Args),
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}
