// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::commands::{bonding_information, build_info, migrate, node_details, run, sign};
use crate::env::vars::{NYMNODE_CONFIG_ENV_FILE_ARG, NYMNODE_NO_BANNER_ARG};
use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use nym_node::error::NymNodeError;
use std::sync::OnceLock;

mod commands;
mod helpers;

pub const DEFAULT_NYMNODE_ID: &str = "default-nym-node";

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the nym-node and overrides any preconfigured values.
    #[clap(
        short,
        long,
        env = NYMNODE_CONFIG_ENV_FILE_ARG
    )]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// Flag used for disabling the printed banner in tty.
    #[clap(
        long,
        env = NYMNODE_NO_BANNER_ARG
    )]
    pub(crate) no_banner: bool,

    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    pub(crate) async fn execute(self) -> Result<(), NymNodeError> {
        match self.command {
            Commands::BuildInfo(args) => build_info::execute(args),
            Commands::BondingInformation(args) => bonding_information::execute(args).await,
            Commands::NodeDetails(args) => node_details::execute(args).await,
            Commands::Run(args) => run::execute(*args).await,
            Commands::Migrate(args) => migrate::execute(*args).await,
            Commands::Sign(args) => sign::execute(args).await,
        }
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Show build information of this binary
    BuildInfo(build_info::Args),

    /// Show bonding information of this node depending on its currently selected mode.
    BondingInformation(bonding_information::Args),

    /// Show details of this node.
    NodeDetails(node_details::Args),

    /// Attempt to migrate an existing mixnode or gateway into a nym-node.
    Migrate(Box<migrate::Args>),

    /// Start this nym-node
    Run(Box<run::Args>),

    /// Use identity key of this node to sign provided message.
    Sign(sign::Args),
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
