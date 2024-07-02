use std::path::PathBuf;
// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only
use crate::error::NetworkManagerError;
use crate::helpers::default_db_file;
use crate::manager::NetworkManager;
use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use std::sync::OnceLock;
use url::Url;

mod build_info;
mod bypass_dkg;
mod initialise_new_network;
mod initialise_post_dkg_network;
mod load_network_details;
mod local_client;
mod local_ecash_apis;
mod local_nodes;
// mod migrate;

#[derive(clap::Args, Debug)]
pub(crate) struct CommonArgs {
    #[clap(long)]
    master_mnemonic: Option<bip39::Mnemonic>,

    #[clap(long)]
    rpc_endpoint: Option<Url>,

    #[clap(long)]
    storage_path: Option<PathBuf>,
}

impl CommonArgs {
    pub(crate) async fn network_manager(self) -> Result<NetworkManager, NetworkManagerError> {
        let storage = self.storage_path.unwrap_or_else(default_db_file);
        NetworkManager::new(storage, self.master_mnemonic, self.rpc_endpoint).await
    }
}

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
            Commands::InitialiseNewNetwork(args) => initialise_new_network::execute(args).await,
            Commands::LoadNetworkDetails(args) => load_network_details::execute(args).await,
            Commands::BypassDkg(args) => bypass_dkg::execute(args).await,
            Commands::InitialisePostDkgNetwork(args) => {
                initialise_post_dkg_network::execute(args).await
            }
            Commands::CreateLocalEcashApis(args) => local_ecash_apis::execute(args).await,
            Commands::BondLocalMixnet(args) => local_nodes::execute(args).await,
            Commands::CreateLocalClient(args) => local_client::execute(args).await,
        }
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Show build information of this binary
    BuildInfo(build_info::Args),

    /// Initialise new testnet network
    InitialiseNewNetwork(initialise_new_network::Args),

    /// Attempt to load testnet network details
    LoadNetworkDetails(load_network_details::Args),

    /// Attempt to bypass the DKG by ovewriting the contract state with pre-generated keys
    BypassDkg(bypass_dkg::Args),

    /// Initialise new network and bypass the DKG.
    /// Equivalent of running `initialise-new-network` and `bypass-dkg` separately.
    InitialisePostDkgNetwork(initialise_post_dkg_network::Args),

    /// Attempt to create brand new network, in post DKG-state, using locally running nym-apis
    CreateLocalEcashApis(local_ecash_apis::Args),

    /// Attempt to bond minimal local mixnet (3 mixnodes + 1 gateways) and output the run commands
    BondLocalMixnet(local_nodes::Args),

    /// Initialise a locally run nym-client, adjust its config and output the run command
    CreateLocalClient(local_client::Args),
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
