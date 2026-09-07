// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! [`NymNodeMixDriver`] — concrete driver running the real
//! [`NymNodeDataPipeline`] for each mix node, with sphinx-in-LP clients.
//!
//! Uses wall-clock [`Instant`] timestamps because [`NymNodeDataPipeline`] is
//! hardcoded to that timestamp type. Manual stepping is therefore disabled.
//!
//! [`NymNodeDataPipeline`]: nym_node::node::lp::data::handler::pipeline::NymNodeDataPipeline

use std::sync::Arc;

use anyhow::Context;
use nym_lp::LpTransportSession;
use nym_lp_data::packet::version;
use nym_test_utils::mocks::async_read_write::mock_io_streams;
use rand::rngs::OsRng;
use tracing::info;

use crate::{
    client::{MixSimClient, nymnode::SimNymClient},
    driver::MixSimDriver,
    node::{
        MixSimNode,
        nymnode::{SimNymNode, SimNymNodeLpIdentity},
    },
    topology::{Topology, directory::Directory},
};

/// Give every ordered pair of nodes a real LP session.
///
/// The simulator has no control plane, so nothing would otherwise dial. This runs the same mutual
/// KKT/PSQ handshake the control plane runs, over an in-memory channel pair instead of TCP, and
/// deposits each half in the node that owns it. From there the data plane is production code: real
/// transport keys, real counters, real replay windows.
///
/// Every pair is established up front rather than on demand, so a node is never mid-handshake once
/// ticking starts.
async fn establish_sessions(identities: &[SimNymNodeLpIdentity]) -> anyhow::Result<()> {
    let mut established = 0;

    for (i, initiator) in identities.iter().enumerate() {
        for responder in identities.iter().skip(i + 1) {
            let (mut initiator_stream, mut responder_stream) = mock_io_streams();

            let initiator_keys = initiator.local_peer.clone();
            let responder_remote = responder.local_peer.as_remote();
            let responder_keys = responder.local_peer.clone();
            let initiator_digests = initiator.local_peer.as_remote().kem_key_digests().clone();

            let (initiator_session, responder_session) = tokio::try_join!(
                async {
                    LpTransportSession::psq_handshake_initiator_mutual_internode(
                        &mut initiator_stream,
                        initiator_keys,
                        responder_remote,
                        version::CURRENT,
                    )?
                    .complete_handshake()
                    .await
                },
                async {
                    LpTransportSession::psq_handshake_responder_mutual(
                        &mut responder_stream,
                        responder_keys,
                        initiator_digests,
                    )
                    .complete_handshake()
                    .await
                },
            )
            .with_context(|| {
                format!(
                    "LP handshake between {} and {} failed",
                    initiator.socket_address, responder.socket_address
                )
            })?;

            // each side keys its session by the address it will send to
            initiator
                .shared_state
                .node_sessions
                .insert_node_session(responder.socket_address.ip(), initiator_session)?;
            responder
                .shared_state
                .node_sessions
                .insert_node_session(initiator.socket_address.ip(), responder_session)?;

            established += 1;
        }
    }

    info!("established {established} LP session(s) between simulated nodes");
    Ok(())
}

/// Concrete [`MixSimDriver`] instantiation that runs the real
/// [`NymNodeDataPipeline`] inside every node and produces sphinx-in-LP packets
/// from every client.
///
/// [`NymNodeDataPipeline`]: nym_node::node::lp::data::handler::pipeline::NymNodeDataPipeline
pub struct NymNodeMixDriver(MixSimDriver);

impl NymNodeMixDriver {
    /// Load a topology JSON file and initialise the driver with one
    /// [`SimNymNode`] per topology node and one [`SimNymClient`] per
    /// topology client.
    ///
    /// [`SimNymNode`]: crate::node::nymnode::SimNymNode
    /// [`SimNymClient`]: crate::client::nymnode::SimNymClient
    pub async fn new(topology: String) -> anyhow::Result<Self> {
        let topology = Topology::load(&topology)?;

        let directory: Arc<Directory> = Arc::new((&topology).into());

        let mut nodes: Vec<Box<dyn MixSimNode + Send>> = Vec::with_capacity(topology.nodes.len());
        let mut identities = Vec::with_capacity(topology.nodes.len());
        for top_node in topology.nodes {
            info!("Setting up node {}", top_node.node_id);
            let (node, identity) = SimNymNode::new(top_node, directory.clone(), OsRng)?;
            nodes.push(Box::new(node));
            identities.push(identity);
        }

        info!("Establishing LP handshakes");
        establish_sessions(&identities).await?;

        let mut clients: Vec<Box<dyn MixSimClient + Send>> =
            Vec::with_capacity(topology.clients.len());
        for top_client in topology.clients {
            let client = SimNymClient::new(top_client, directory.clone(), OsRng)?;
            clients.push(Box::new(client));
        }

        Ok(NymNodeMixDriver(MixSimDriver::new(nodes, clients)))
    }

    /// Run the simulation; delegates to [`MixSimDriver::run`].
    ///
    pub async fn run(
        self,
        manual_mode: bool,
        display_state: bool,
        tick_duration_ms: u64,
    ) -> anyhow::Result<()> {
        self.0
            .run(manual_mode, display_state, tick_duration_ms)
            .await
    }
}
