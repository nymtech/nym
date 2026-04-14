// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use crate::cli::common::CommonArgs;
use std::net::IpAddr;
use tracing::error;
use url::Url;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    common_args: CommonArgs,

    /// Address of the orchestrator for requesting work assignments
    #[clap(long, env = NYM_NETWORK_MONITOR_AGENT_ORCHESTRATOR_ADDRESS_ARG)]
    orchestrator_address: Url,

    /// Bearer token required for requesting work assignments
    /// and submitting the results
    #[clap(long, env = NYM_NETWORK_MONITOR_AGENT_ORCHESTRATOR_TOKEN_ARG)]
    orchestrator_token: String,

    /// Egress IP address of this agent, retrieved from status.hostIP via the Downward API
    #[clap(long, env = NYM_NETWORK_MONITOR_AGENT_HOST_IP_ARG)]
    host_ip: IpAddr,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    let _ = args;
    error!("this command hasn't been implemented yet");
    Ok(())
}
