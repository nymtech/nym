// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use crate::agent::NetworkMonitorAgent;
use crate::agent::helpers::load_noise_key;
use crate::cli::common::CommonArgs;
use nym_network_monitor_orchestrator_requests::client::OrchestratorClient;
use std::net::{IpAddr, SocketAddr};
use tracing::info;
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

    /// Announced port of this agent, used alongside host_ip by nodes sending packets back to the agent
    #[clap(long, env = NYM_NETWORK_MONITOR_AGENT_HOST_PORT_ARG)]
    host_port: u16,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    let orchestrator_client =
        OrchestratorClient::new(args.orchestrator_address.into(), args.orchestrator_token)?;

    let noise_key = load_noise_key(&args.common_args.noise_key_path)?;

    let external_address = SocketAddr::new(args.host_ip, args.host_port);

    // 1. build instance of the agent (loads the noise keys)
    let agent = NetworkMonitorAgent::new(
        args.common_args.build_config(external_address)?,
        noise_key,
        orchestrator_client,
    );

    // 2. announce the agent to the orchestrator
    // so that it would be registered in the smart contract
    // (if it hasn't been announced before)
    info!("announcing agent information to the orchestrator");
    agent.announce_agent().await?;

    // 3. query the orchestrator for work assignment and attempt to perform the stress test
    // of the target node
    info!("attempting to request test run assignment");
    agent.run_stress_test().await?;

    Ok(())
}
