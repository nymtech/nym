// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use crate::agent::NetworkMonitorAgent;
use crate::agent::config::Config;
use crate::agent::tested_node::TestedNodeDetails;
use crate::cli::common::CommonArgs;
use nym_crypto::asymmetric::x25519;
use nym_sphinx_params::SphinxKeyRotation;
use std::net::SocketAddr;
use tracing::info;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    common_args: CommonArgs,

    /// The socket address of the agent to use for receiving test packets back
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_MIXNET_ADDRESS_ARG)]
    agent_mixnet_listener: SocketAddr,

    /// The socket address of the node to test
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_NODE_ADDRESS_ARG)]
    tested_node_address: SocketAddr,

    /// Noise key of the node to test
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_NODE_NOISE_KEY_ARG)]
    tested_node_noise_key: x25519::PublicKey,

    /// Sphinx key of the node to test
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_NODE_SPHINX_KEY_ARG)]
    tested_node_sphinx_key: x25519::PublicKey,

    /// Current sphinx key rotation of the node to test
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_NODE_SPHINX_KEY_ROTATION_ARG)]
    tested_node_sphinx_key_rotation: u32,
}

impl Args {
    pub(crate) fn build_agent_config(&self) -> Config {
        self.common_args.build_config(self.agent_mixnet_listener)
    }

    pub(crate) fn build_tested_node_details(&self) -> TestedNodeDetails {
        TestedNodeDetails {
            address: self.tested_node_address,
            noise_key: self.tested_node_noise_key,
            sphinx_key: self.tested_node_sphinx_key,
            key_rotation: SphinxKeyRotation::from_key_rotation_id(
                self.tested_node_sphinx_key_rotation,
            ),
        }
    }

    pub(crate) fn build_agent(&self) -> anyhow::Result<NetworkMonitorAgent> {
        NetworkMonitorAgent::new(
            self.build_agent_config(),
            &self.common_args.noise_key_path,
            self.build_tested_node_details(),
        )
    }
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    let result = args.build_agent()?.run_stress_test().await?;

    info!("{result:#?}");
    Ok(())
}
