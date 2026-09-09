// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use nym_network_monitor_orchestrator_requests::models::TestKind;
use std::sync::OnceLock;

mod build_info;
mod common;
mod discover;
mod env;
mod keygen;
mod manual;
mod run_agent;
mod test_gateway_liveness;
mod test_mixnode;

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

/// Top-level CLI entry point for the network monitor agent.
#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

impl Cli {
    /// Dispatches execution to the subcommand selected by the user.
    pub(crate) async fn execute(self) -> anyhow::Result<()> {
        match self.command {
            Command::BuildInfo(args) => build_info::execute(args),
            Command::TestMixnodeStress(args) => {
                test_mixnode::execute(args, TestKind::Stress).await?
            }
            Command::TestMixnodeLiveness(args) => {
                test_mixnode::execute(args, TestKind::Liveness).await?
            }
            Command::TestGatewayLiveness(args) => test_gateway_liveness::execute(args).await?,
            Command::RunAgent(args) => run_agent::execute(args).await?,
            Command::Keygen(args) => keygen::execute(args)?,
        }
        Ok(())
    }
}

/// The manual `test-*` commands are named for the (kind, role) pair they exercise, matching the
/// [`TestRunAssignment`](nym_network_monitor_orchestrator_requests::models::TestRunAssignment)
/// variants an orchestrator would hand out. There are exactly three, since a gateway is probed only
/// for liveness.
#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Show build information of this binary
    BuildInfo(build_info::Args),

    /// One-shot manual STRESS test of a mixnode, without interacting with the orchestrator
    TestMixnodeStress(test_mixnode::Args),

    /// One-shot manual LIVENESS test of a mixnode, without interacting with the orchestrator
    TestMixnodeLiveness(test_mixnode::Args),

    /// One-shot manual LIVENESS test of an entry gateway, exercising both its client ingest and its
    /// client delivery, without interacting with the orchestrator
    TestGatewayLiveness(test_gateway_liveness::Args),

    /// Test a node by contacting the orchestrator for the work assignment
    RunAgent(run_agent::Args),

    /// Generate all required keys for the agent to work
    Keygen(keygen::Args),
}
