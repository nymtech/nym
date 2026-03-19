// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use std::sync::OnceLock;

mod build_info;
mod common;
mod env;
mod keygen;
mod run_agent;
mod test_node;

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

impl Cli {
    pub(crate) async fn execute(self) -> anyhow::Result<()> {
        match self.command {
            Command::BuildInfo(args) => build_info::execute(args),
            Command::TestNode(args) => test_node::execute(args).await?,
            Command::RunAgent(args) => run_agent::execute(args).await?,
            Command::Keygen(args) => keygen::execute(args)?,
        }
        Ok(())
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Show build information of this binary
    BuildInfo(build_info::Args),

    /// One-shot manual testing of a specified node
    /// without interacting with the orchestrator.
    TestNode(test_node::Args),

    /// Test a node by contacting the orchestrator for the work assignment
    RunAgent(run_agent::Args),

    /// Generate all required keys for the agent to work
    Keygen(keygen::Args),
}
