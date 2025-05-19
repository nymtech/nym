// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::commands::{
    bonding_information, build_info, migrate, node_details, reset_sphinx_keys, run, sign,
    test_throughput,
};
use crate::env::vars::{NYMNODE_CONFIG_ENV_FILE_ARG, NYMNODE_NO_BANNER_ARG};
use crate::logging::setup_tracing_logger;
use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use std::future::Future;
use std::sync::OnceLock;

pub(crate) mod commands;
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
    fn execute_async<F: Future>(fut: F) -> anyhow::Result<F::Output> {
        Ok(tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?
            .block_on(fut))
    }

    pub(crate) fn execute(self) -> anyhow::Result<()> {
        // NOTE: `test_throughput` sets up its own logger as it has to include additional layers
        if !matches!(self.command, Commands::TestThroughput(..)) {
            setup_tracing_logger()?;
        }

        match self.command {
            Commands::BuildInfo(args) => build_info::execute(args)?,
            Commands::BondingInformation(args) => {
                { Self::execute_async(bonding_information::execute(args))? }?
            }
            Commands::NodeDetails(args) => { Self::execute_async(node_details::execute(args))? }?,
            Commands::Run(args) => { Self::execute_async(run::execute(*args))? }?,
            Commands::Migrate(args) => migrate::execute(*args)?,
            Commands::Sign(args) => { Self::execute_async(sign::execute(args))? }?,
            Commands::TestThroughput(args) => test_throughput::execute(args)?,
            Commands::UnsafeResetSphinxKeys(args) => {
                { Self::execute_async(reset_sphinx_keys::execute(args))? }?
            }
        }
        Ok(())
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

    /// UNSAFE: reset existing sphinx keys and attempt to generate fresh one for the current network state
    UnsafeResetSphinxKeys(reset_sphinx_keys::Args),

    /// Attempt to approximate the maximum mixnet throughput if nym-node
    /// was running on this machine in mixnet mode
    #[clap(hide = true)]
    TestThroughput(test_throughput::Args),
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
