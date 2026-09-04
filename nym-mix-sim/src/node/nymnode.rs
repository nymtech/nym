// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! [`SimNymNode`] — mix node that runs the real [`NymNodeDataPipeline`].

use std::net::SocketAddr;
use std::sync::Arc;

use getrandom04::SysRng;
use nym_lp::peer::{LpLocalPeer, random_peer};
use nym_lp_data::packet::{EncryptedLpPacket, LpFrame};
use nym_node::node::lp::data::{
    handler::pipeline::NymNodeDataPipeline,
    shared::{SharedGatewayLpDataState, SharedLpDataState},
};
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use rand::Rng;
use rand010::SeedableRng;

use crate::{
    node::BaseNode,
    topology::{TopologyNode, directory::Directory},
};

/// A simulated mix node driven by the real [`NymNodeDataPipeline`].
///
/// This is a type alias for [`BaseNode`] specialised to [`EncryptedLpPacket`]
/// and [`NymNodeDataPipeline`]. All tick logic lives in the generic
/// [`MixSimNode`] impl on `BaseNode`; routing produces [`NymNodeRoutingAddress`]es
/// which the transport wrap resolves to socket addresses.
///
/// [`MixSimNode`]: crate::node::MixSimNode
pub type SimNymNode<R> =
    BaseNode<EncryptedLpPacket, LpFrame, NymNodeDataPipeline<R>, NymNodeRoutingAddress>;

/// What a node has to expose for the driver to pair it with its peers.
///
/// The handshake needs the local keys and the address the peer will be reached at; storing the
/// session afterwards needs the state the data plane reads.
pub struct SimNymNodeLpIdentity {
    pub local_peer: LpLocalPeer,
    pub shared_state: Arc<SharedLpDataState>,
    pub socket_address: SocketAddr,
}

impl<R: Rng + Send> SimNymNode<R> {
    /// Create a [`SimNymNode`] from a [`TopologyNode`] description by binding
    /// a non-blocking UDP socket to `node.socket_address` and constructing a
    /// simulation-ready [`NymNodeDataPipeline`] with the node's sphinx key.
    ///
    /// Returns the node alongside the identity the driver needs to hand it sessions.
    ///
    /// # Errors
    ///
    /// Returns an error if the UDP socket cannot be bound or set non-blocking.
    pub fn new(
        topology_node: TopologyNode,
        directory: Arc<Directory>,
        rng: R,
    ) -> anyhow::Result<(Self, SimNymNodeLpIdentity)> {
        let shared = Arc::new(SharedLpDataState::new_for_simulation(
            topology_node.sphinx_private_key,
            directory.as_client_map(),
        ));
        let gateway = Arc::new(SharedGatewayLpDataState::new_for_simulation(
            directory.as_nym_topology(),
        ));

        // LP keys are generated per run rather than carried in the topology file: the simulation
        // needs no identity across runs, and an ML-KEM768 keypair would be several kilobytes of
        // JSON per node.
        let mut key_rng = rand010::rngs::StdRng::try_from_rng(&mut SysRng)?;
        let local_peer = random_peer(&mut key_rng);

        let identity = SimNymNodeLpIdentity {
            local_peer: local_peer.clone(),
            shared_state: shared.clone(),
            socket_address: topology_node.socket_address,
        };

        let pipeline = NymNodeDataPipeline::new(shared, gateway, rng);

        let node = BaseNode::with_pipeline(
            topology_node.node_id,
            topology_node.reliability,
            topology_node.socket_address,
            pipeline,
        )?;

        Ok((node, identity))
    }
}
