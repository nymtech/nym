// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! [`SphinxNode`] — mix node using the full Sphinx packet pipeline.

use std::{sync::Arc, time::Instant};

use nym_crypto::asymmetric::x25519;
use nym_lp_data::{
    AddressedTimedData, AddressedTimedPayload, TimedPayload,
    common::helpers::{NoOpWireUnwrapper, NoOpWireWrapper},
    nymnodes::traits::NymNodeProcessingPipeline,
};
use nym_sphinx::SphinxPacket;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;

use crate::{
    node::{BaseNode, NodeId},
    packet::{
        WirePacketFormat,
        sphinx::{SimMixPacket, SurbAck},
    },
    topology::{TopologyNode, directory::Directory},
};

/// A mix-node that uses the Sphinx packet pipeline.
///
/// This is a type alias for [`BaseNode`] specialised to [`SimMixPacket`] and
/// [`SphinxProcessingNode`].  All tick logic lives in the generic
/// [`MixSimNode`] impl on `BaseNode`.
///
/// [`MixSimNode`]: crate::node::MixSimNode
pub type SphinxNode = BaseNode<SimMixPacket, Vec<u8>, SphinxProcessingNode>;

impl SphinxNode {
    /// Create a [`SphinxNode`] from a [`TopologyNode`] description by binding a
    /// non-blocking UDP socket to `node.socket_address`.
    ///
    /// # Errors
    ///
    /// Returns an error if the UDP socket cannot be bound or set non-blocking.
    pub fn new(topology_node: TopologyNode, directory: Arc<Directory>) -> anyhow::Result<Self> {
        let pipeline = SphinxProcessingNode::new(
            topology_node.node_id,
            topology_node.sphinx_private_key,
            directory,
        );
        BaseNode::with_pipeline(
            topology_node.node_id,
            topology_node.reliability,
            topology_node.socket_address,
            pipeline,
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// A [`NymNodeProcessingPipeline`] for [`SphinxPacket`].
///
/// Uses no-op framing and transport wrappers because a Sphinx packet is already
/// its own self-contained wire unit — no additional framing or transport header
/// is needed.  The real work happens in [`mix`](SphinxProcessingNode::mix), which
/// peels one onion layer with the node's private key and extracts the next-hop
/// address and per-hop delay.
pub struct SphinxProcessingNode {
    id: NodeId,
    sphinx_secret: x25519::PrivateKey,
    directory: Arc<Directory>,
}

impl SphinxProcessingNode {
    /// Construct a pipeline for the node identified by `node_id`, using
    /// `sphinx_secret` to decrypt incoming Sphinx packets.
    pub fn new(
        node_id: NodeId,
        sphinx_secret: x25519::PrivateKey,
        directory: Arc<Directory>,
    ) -> Self {
        Self {
            id: node_id,
            sphinx_secret,
            directory,
        }
    }
}

impl NymNodeProcessingPipeline<Vec<u8>> for SphinxProcessingNode {
    /// No-op wire layer: frames are raw payloads, so the whole packet budget is available.
    fn frame_size(&self) -> usize {
        <Self as nym_lp_data::common::helpers::NoOpWireWrapper>::PACKET_SIZE
    }

    type MessageKind = ();
    type Options = ();

    /// Peel one Sphinx layer and forward or deliver the result.
    ///
    /// - **ForwardHop**: extracts the next-hop packet, address (byte 0 of the
    ///   32-byte address field encodes the [`NodeId`]), and per-hop delay; schedules
    ///   the re-wrapped packet at `timestamp + delay`.
    /// - **FinalHop**: delivers the plaintext payload directly to the destination
    ///   client (identified by byte 0 of the destination address).
    fn mix(
        &mut self,
        _: (),
        payload: TimedPayload,
        timestamp: Instant,
    ) -> Vec<AddressedTimedPayload> {
        // SAFETY: Given the no-op unwrapper used here, payload.data is always a
        // valid serialised SphinxPacket at this point.
        #[allow(clippy::unwrap_used)]
        let sphinx_packet = SphinxPacket::from_bytes(&payload.data).unwrap();

        match sphinx_packet.process(self.sphinx_secret.inner()) {
            Ok(packet) => match packet.data {
                nym_sphinx::ProcessedPacketData::ForwardHop {
                    next_hop_packet,
                    next_hop_address,
                    delay,
                } => {
                    let Ok(routing_address) = next_hop_address.try_into() else {
                        tracing::warn!("[Node {}] Cannot recover routing address", self.id);
                        return Vec::new();
                    };

                    let NymNodeRoutingAddress::Node(next_hop_address) = routing_address else {
                        tracing::warn!(
                            "[Node {}] Received a sphinx packet with a Client routing address",
                            self.id
                        );
                        return Vec::new();
                    };
                    let timed_sphinx = AddressedTimedData::new_addressed(
                        timestamp + delay.to_duration(),
                        next_hop_packet.to_bytes(),
                        next_hop_address,
                    );
                    vec![timed_sphinx]
                }
                nym_sphinx::ProcessedPacketData::FinalHop {
                    destination,
                    identifier: _,
                    payload,
                } => {
                    let mut packets_to_forward = Vec::new();

                    if let Ok(plaintext) = payload
                        .recover_plaintext()
                        .inspect_err(|e| tracing::warn!("Impossible to recover plaintext : {e}"))
                    {
                        let (surb_ack_bytes, message) = SurbAck::extract_ack_and_message(plaintext);

                        // Client packet handling
                        if let Some(client_socket_address) = self
                            .directory
                            .client(destination.as_bytes()[0])
                            .map(|n| n.addr)
                        {
                            packets_to_forward.push(AddressedTimedData::new_addressed(
                                timestamp,
                                message,
                                client_socket_address,
                            ));
                        } else {
                            tracing::warn!(
                                "[Node {}] Client {} not found in the directory",
                                self.id,
                                destination.as_bytes()[0]
                            );
                        };

                        // SURB_ACK handling
                        if !surb_ack_bytes.is_empty()
                            && let Ok((next_hop, surb_ack)) = SurbAck::try_recover_first_hop_packet(
                                &surb_ack_bytes,
                            )
                            .inspect_err(|e| tracing::warn!("Fail to deserialize SURB Ack : {e}"))
                        {
                            if let Some(next_hop_socket_address) =
                                self.directory.node(next_hop).map(|n| n.addr)
                            {
                                packets_to_forward.push(AddressedTimedData::new_addressed(
                                    timestamp,
                                    surb_ack.to_bytes(),
                                    next_hop_socket_address,
                                ));
                            } else {
                                tracing::warn!("Node {next_hop} not found in the directory",);
                            }
                        }
                    }

                    packets_to_forward
                }
            },
            Err(e) => {
                tracing::error!("[Node {}] Failed to process a sphinx packet : {e}", self.id);
                Vec::new()
            }
        }
    }
}

// Boilerplate subtraits delegation
impl NoOpWireWrapper for SphinxProcessingNode {}
impl NoOpWireUnwrapper for SphinxProcessingNode {}
