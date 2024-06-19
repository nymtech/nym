// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only
use crate::error::NetworkManagerError;
use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use std::sync::OnceLock;

mod build_info;
mod bypass_dkg;
mod initialise_new_network;
mod load_network_details;

// Helper for passing LONG_VERSION to clap
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
    pub(crate) async fn execute(self) -> Result<(), NetworkManagerError> {
        match self.command {
            Commands::BuildInfo(args) => build_info::execute(args),
            Commands::InitialiseNewNetwork(args) => initialise_new_network::execute(*args).await,
            Commands::LoadNetworkDetails(args) => load_network_details::execute(args).await,
            Commands::BypassDkg(args) => bypass_dkg::execute(args).await,
        }
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Show build information of this binary
    BuildInfo(build_info::Args),

    /// Initialise new testnet network
    InitialiseNewNetwork(Box<initialise_new_network::Args>),

    /// Attempt to load testnet network details
    LoadNetworkDetails(load_network_details::Args),

    /// Attempt to bypass the DKG by ovewriting the contract state with pre-generated keys
    BypassDkg(bypass_dkg::Args),
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
