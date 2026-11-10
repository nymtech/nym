// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! What every manual `test-*` command needs: where to aim, and the keys to aim with.

use super::env::vars::*;
use crate::agent::config::NodeTesterConfig;
use crate::agent::helpers::{derive_client_identity, load_noise_key};
use crate::cli::common::CommonArgs;
use crate::cli::discover::DiscoveredNode;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_sphinx_types::DestinationAddressBytes;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, warn};

/// The node a manual run targets, plus the local addressing it is reached from.
///
/// The node is named by its HTTP api alone: everything a probe needs beyond that (the identity, the
/// noise and sphinx keys, the key rotation, the mix port, the announced addresses, the client
/// websocket port) is read off the node itself by [`DiscoveredNode`]. Passing them in by hand is how
/// a run ends up aimed with a rotated sphinx key and diagnosed as a dead node.
#[derive(clap::Args, Debug)]
pub(crate) struct ManualTargetArgs {
    #[clap(flatten)]
    pub(crate) common_args: CommonArgs,

    /// Host of the node to test, as an ip or a hostname. Its keys, ports and roles are read from its
    /// http api.
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_NODE_HOST_ARG)]
    tested_node_host: String,

    /// Port the node serves its http api on, if it is not one of the standard ones.
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_NODE_HTTP_PORT_ARG)]
    tested_node_http_port: Option<u16>,

    /// The ipv4 socket address this agent receives test packets back on.
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_MIXNET_ADDRESS_V4_ARG)]
    agent_mixnet_listener_v4: SocketAddr,

    /// The ipv6 socket address this agent receives test packets back on.
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_MIXNET_ADDRESS_V6_ARG)]
    agent_mixnet_listener_v6: SocketAddr,
}

/// The keys a manual run probes with, derived exactly as the agent proper derives them.
pub(crate) struct ManualKeys {
    pub(crate) noise_key: Arc<x25519::KeyPair>,

    /// Announced on chain in a real deployment. A locally run node has to have been told about this
    /// identity for a gateway probe's session to be granted the monitor exemption, which is why
    /// [`load_keys`](ManualTargetArgs::load_keys) logs it.
    pub(crate) client_identity: Arc<ed25519::KeyPair>,
}

impl ManualKeys {
    /// The client address every test packet is addressed to.
    pub(crate) fn client_address(&self) -> DestinationAddressBytes {
        self.client_identity
            .public_key()
            .derive_destination_address()
    }
}

impl ManualTargetArgs {
    /// Builds the agent [`NodeTesterConfig`] from the common args and this agent's listener pair.
    pub(crate) fn build_tester_config(&self) -> anyhow::Result<NodeTesterConfig> {
        let config = self
            .common_args
            .build_config(self.agent_mixnet_listener_v4, self.agent_mixnet_listener_v6)?;

        // a deployment legitimately announces a different port from the one it binds, because a host
        // port mapping sits in between. a manual run has nothing in between, so a mismatch means the
        // tested node dials a port nothing is listening on and every probe times out with no
        // indication of why - the packet leaves, and silence is the only symptom. warned rather than
        // refused, since a local forward is unusual but not wrong
        let announced = config.announced_addresses();
        let bound = config.mixnet_bind_address.port();
        if announced.v4.port() != bound || announced.v6.port() != bound {
            warn!(
                "this agent is BINDING {} but ANNOUNCING {} / {}: unless something forwards those ports, the tested node will send its packets where nothing is listening and every probe will time out. pass --bind-address to match",
                config.mixnet_bind_address, announced.v4, announced.v6
            );
        }

        Ok(config)
    }

    /// Reads the target node's keys, ports and roles off its http api.
    pub(crate) async fn discover(&self) -> anyhow::Result<DiscoveredNode> {
        DiscoveredNode::query(&self.tested_node_host, self.tested_node_http_port).await
    }

    /// Loads the noise key and derives the client identity from it.
    ///
    /// Both are logged, because a locally run node will not treat this agent as a monitor until it
    /// has been told about them: the mixnet gates key on the announced ADDRESS and the gateway
    /// session gate on the announced IDENTITY, so a probe against a node that knows neither scores
    /// zero for reasons that have nothing to do with the node.
    pub(crate) fn load_keys(&self) -> anyhow::Result<ManualKeys> {
        let noise_key = load_noise_key(&self.common_args.noise_key_path)?;
        let client_identity = Arc::new(derive_client_identity(&noise_key)?);

        info!(
            "probing as {} from {} / {}",
            client_identity.public_key(),
            self.agent_mixnet_listener_v4,
            self.agent_mixnet_listener_v6
        );
        info!(
            "the tested node must authorise this agent's address, and for a gateway probe also its identity, or the run will score zero"
        );

        Ok(ManualKeys {
            noise_key,
            client_identity,
        })
    }
}
