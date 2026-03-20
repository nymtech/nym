// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use std::sync::OnceLock;

mod build_info;
mod env;
mod run_orchestrator;

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

/// Top-level CLI entry point for the network monitor agent.
#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the binary.
    /// Useful in local testing setups against networks different from mainnet
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[command(subcommand)]
    pub(crate) command: Command,
}

impl Cli {
    /// Dispatches execution to the subcommand selected by the user.
    pub(crate) async fn execute(self) -> anyhow::Result<()> {
        match self.command {
            Command::BuildInfo(args) => build_info::execute(args),
            Command::RunOrchestrator(args) => run_orchestrator::execute(args).await?,
        }
        Ok(())
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Show build information of this binary
    BuildInfo(build_info::Args),

    /// Run the network monitor orchestrator which will periodically
    /// issue work assignments for stress testing mixnodes
    RunOrchestrator(run_orchestrator::Args),
}
