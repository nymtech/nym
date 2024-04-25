// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::Cli;
use clap::CommandFactory;
use clap::Subcommand;
use log::warn;
use nym_bin_common::completions::{fig_generate, ArgShell};
use std::io::IsTerminal;
use std::time::Duration;

pub(crate) mod build_info;
pub(crate) mod helpers;
pub(crate) mod init;
pub(crate) mod node_details;
pub(crate) mod run;
pub(crate) mod setup_ip_packet_router;
pub(crate) mod setup_network_requester;
pub(crate) mod sign;
mod upgrade_helpers;

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Initialise the gateway
    Init(init::Init),

    /// Show details of this gateway
    NodeDetails(node_details::NodeDetails),

    /// Starts the gateway
    Run(Box<run::Run>),

    /// Add network requester support to this gateway
    // essentially an option to include NR without having to setup fresh gateway
    SetupNetworkRequester(setup_network_requester::CmdArgs),

    /// Add ip packet router support to this gateway
    // essentially an option to include ip packet router without having to setup fresh gateway
    #[command(hide = true)]
    SetupIpPacketRouter(setup_ip_packet_router::CmdArgs),

    /// Sign text to prove ownership of this mixnode
    Sign(sign::Sign),

    /// Show build information of this binary
    BuildInfo(build_info::BuildInfo),

    /// Generate shell completions
    Completions(ArgShell),

    /// Generate Fig specification
    GenerateFigSpec,
}

pub(crate) async fn execute(args: Cli) -> anyhow::Result<()> {
    let bin_name = "nym-gateway";

    warn!("standalone gateways have been deprecated - please consider migrating it to a `nym-node` via `nym-node migrate gateway` command");
    if std::io::stdout().is_terminal() {
        // if user is running it in terminal session,
        // introduce the delay, so they'd notice the message
        tokio::time::sleep(Duration::from_secs(1)).await
    }

    match args.command {
        Commands::Init(m) => init::execute(m).await?,
        Commands::NodeDetails(m) => node_details::execute(m).await?,
        Commands::Run(m) => run::execute(*m).await?,
        Commands::SetupNetworkRequester(m) => setup_network_requester::execute(m).await?,
        Commands::SetupIpPacketRouter(m) => setup_ip_packet_router::execute(m).await?,
        Commands::Sign(m) => sign::execute(m)?,
        Commands::BuildInfo(m) => build_info::execute(m),
        Commands::Completions(s) => s.generate(&mut crate::Cli::command(), bin_name),
        Commands::GenerateFigSpec => fig_generate(&mut crate::Cli::command(), bin_name),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}
