// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use crate::agent::NetworkMonitorAgent;
use crate::agent::helpers::load_noise_key;
use crate::cli::common::CommonArgs;
use nym_network_monitor_orchestrator_requests::client::OrchestratorClient;
use nym_task::{ShutdownManager, ShutdownToken};
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

    /// Ipv4 Egress IP address of this agent
    #[clap(long, env = NYM_NETWORK_MONITOR_AGENT_HOST_IPV4_ARG)]
    host_ip_v4: IpAddr,

    /// Ipv6 Egress IP address of this agent
    #[clap(long, env = NYM_NETWORK_MONITOR_AGENT_HOST_IPV6_ARG)]
    host_ip_v6: IpAddr,

    /// Announced port of this agent, used alongside host_ip by nodes sending packets back to the agent
    #[clap(long, env = NYM_NETWORK_MONITOR_AGENT_HOST_PORT_ARG)]
    host_port: u16,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    let mut shutdown_manager = ShutdownManager::build_new_default()?;

    let shutdown = shutdown_manager.clone_shutdown_token();
    let agent_future = build_and_run_agent(args, shutdown);

    tokio::select! {
        // a signal arrived first: `run_until_shutdown` has already cancelled the root token, which is
        // what stops the wave's listener and every session it holds open. NOT a failure of the run
        _ = shutdown_manager.run_until_shutdown() => {
            info!("shut down before the assignment finished");
            Ok(())
        }
        // the assignment finished on its own, and its outcome is the PROCESS's outcome: this is a
        // one-shot job, so a swallowed error here would exit zero on a run that never tested anything
        result = agent_future => result,
    }
}

async fn build_and_run_agent(args: Args, shutdown: ShutdownToken) -> anyhow::Result<()> {
    let orchestrator_client =
        OrchestratorClient::new(args.orchestrator_address.into(), args.orchestrator_token)?;

    let noise_key = load_noise_key(&args.common_args.noise_key_path)?;

    let external_address_v4 = SocketAddr::new(args.host_ip_v4, args.host_port);
    let external_address_v6 = SocketAddr::new(args.host_ip_v6, args.host_port);

    // 1. build instance of the agent (loads the noise keys and derives the client identity)
    let agent = NetworkMonitorAgent::new(
        args.common_args
            .build_config(external_address_v4, external_address_v6)?,
        noise_key,
        orchestrator_client,
        shutdown,
    )?;

    // 2. announce the agent to the orchestrator
    // so that it would be registered in the smart contract
    // (if it hasn't been announced before)
    info!("announcing agent information to the orchestrator");
    agent.announce_agent().await?;

    // 3. query the orchestrator for work assignment and attempt to perform the stress test
    // of the target node
    info!("attempting to request test run assignment");
    agent.perform_work_assignment().await?;

    Ok(())
}
