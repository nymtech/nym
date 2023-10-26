// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Cli;
use clap::CommandFactory;
use clap::Subcommand;
use nym_bin_common::completions::{fig_generate, ArgShell};
use std::error::Error;

pub(crate) mod build_info;
pub(crate) mod helpers;
pub(crate) mod init;
pub(crate) mod node_details;
pub(crate) mod run;
pub(crate) mod setup_ip_forwarder;
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
    Run(run::Run),

    /// Add network requester support to this gateway
    // essentially an option to include NR without having to setup fresh gateway
    SetupNetworkRequester(setup_network_requester::CmdArgs),

    /// Add ip forwarder support to this gateway
    // essentially an option to include ip forwarder without having to setup fresh gateway
    SetupIpForwarder(setup_ip_forwarder::CmdArgs),

    /// Sign text to prove ownership of this mixnode
    Sign(sign::Sign),

    /// Show build information of this binary
    BuildInfo(build_info::BuildInfo),

    /// Generate shell completions
    Completions(ArgShell),

    /// Generate Fig specification
    GenerateFigSpec,
}

pub(crate) async fn execute(args: Cli) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bin_name = "nym-gateway";

    match args.command {
        Commands::Init(m) => init::execute(m).await?,
        Commands::NodeDetails(m) => node_details::execute(m).await?,
        Commands::Run(m) => run::execute(m).await?,
        Commands::SetupNetworkRequester(m) => setup_network_requester::execute(m).await?,
        Commands::SetupIpForwarder(m) => setup_ip_forwarder::execute(m).await?,
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
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}
