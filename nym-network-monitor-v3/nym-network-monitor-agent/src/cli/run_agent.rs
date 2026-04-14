// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use crate::agent::NetworkMonitorAgent;
use crate::cli::common::CommonArgs;
use nym_network_monitor_orchestrator_requests::client::OrchestratorClient;
use nym_network_monitor_orchestrator_requests::models::AgentPortRequest;
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
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    let orchestrator_client =
        OrchestratorClient::new(args.orchestrator_address.into(), args.orchestrator_token)?;

    // 1. retrieve mix port to use as coordinated by the orchestrator
    // to make sure there are no two agents with the same egress ip address and port
    // as otherwise nodes would be unable to determine correct noise key to use
    info!("requesting mix port assignment");
    let mix_port = orchestrator_client
        .get_mix_port_assignment(&AgentPortRequest {
            agent_node_ip: args.host_ip,
        })
        .await?
        .available_mix_port;

    let mix_address = SocketAddr::new(args.host_ip, mix_port);

    // 2. build instance of the agent (loads the noise keys)
    let agent = NetworkMonitorAgent::new(
        args.common_args.build_config(mix_address),
        args.common_args.noise_key_path,
        orchestrator_client,
    )?;

    // 3. announce the agent to the orchestrator
    // so that it would be registered in the smart contract
    // (if it hasn't been announced before)
    info!("announcing agent information to the orchestrator");
    agent.announce_agent().await?;

    // 4. query the orchestrator for work assignment and attempt to perform the stress test
    // of the target node
    info!("attempting to request test run assignment");
    agent.run_stress_test().await?;

    Ok(())
}
