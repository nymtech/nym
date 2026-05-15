// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! [`SphinxNode`] — mix node using the full Sphinx packet pipeline.

use std::sync::Arc;

use nym_crypto::asymmetric::x25519;
use nym_lp_data::{
    AddressedTimedData, AddressedTimedPayload, TimedData, TimedPayload,
    common::helpers::{NoOpWireUnwrapper, NoOpWireWrapper},
    mixnodes::traits::MixnodeProcessingPipeline,
};
use nym_sphinx::SphinxPacket;

use crate::{
    node::{BaseNode, NodeId, ProcessingNode},
    packet::{
        WirePacketFormat,
        sphinx::{AddDelay, SimMixPacket, SphinxMessage, SurbAck},
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
pub type SphinxNode<Ts> = BaseNode<Ts, SimMixPacket, SphinxProcessingNode>;

impl<Ts> SphinxNode<Ts> {
    /// Create a [`SphinxNode`] from a [`TopologyNode`] description by binding a
    /// non-blocking UDP socket to `node.socket_address`.
    ///
    /// # Errors
    ///
    /// Returns an error if the UDP socket cannot be bound or set non-blocking.
    pub fn new(topology_node: TopologyNode, directory: Arc<Directory>) -> anyhow::Result<Self> {
        let pipeline =
            SphinxProcessingNode::new(topology_node.node_id, topology_node.sphinx_private_key);
        BaseNode::with_pipeline(
            topology_node.node_id,
            topology_node.reliability,
            topology_node.socket_address,
            directory,
            pipeline,
        )
    }
}

impl<Ts> ProcessingNode<Ts, SimMixPacket> for SphinxProcessingNode
where
    Ts: AddDelay + Clone,
{
    fn process(
        &mut self,
        input: TimedData<Ts, SimMixPacket>,
        timestamp: Ts,
    ) -> anyhow::Result<Vec<AddressedTimedData<Ts, SimMixPacket, NodeId>>> {
        Ok(MixnodeProcessingPipeline::<
            Ts,
            SimMixPacket,
            SphinxMessage,
            NodeId,
        >::process(self, input, timestamp)?)
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// A [`MixnodeProcessingPipeline`] for [`SphinxPacket`].
///
/// Uses no-op framing and transport wrappers because a Sphinx packet is already
/// its own self-contained wire unit — no additional framing or transport header
/// is needed.  The real work happens in [`mix`](SphinxProcessingNode::mix), which
/// peels one onion layer with the node's private key and extracts the next-hop
/// address and per-hop delay.
pub struct SphinxProcessingNode {
    id: NodeId,
    sphinx_secret: x25519::PrivateKey,
}

impl SphinxProcessingNode {
    /// Construct a pipeline for the node identified by `node_id`, using
    /// `sphinx_secret` to decrypt incoming Sphinx packets.
    pub fn new(node_id: NodeId, sphinx_secret: x25519::PrivateKey) -> Self {
        Self {
            id: node_id,
            sphinx_secret,
        }
    }
}

impl<Ts> MixnodeProcessingPipeline<Ts, SimMixPacket, SphinxMessage, NodeId> for SphinxProcessingNode
where
    Ts: AddDelay + Clone,
{
    /// Peel one Sphinx layer and forward or deliver the result.
    ///
    /// - **ForwardHop**: extracts the next-hop packet, address (byte 0 of the
    ///   32-byte address field encodes the [`NodeId`]), and per-hop delay; schedules
    ///   the re-wrapped packet at `timestamp + delay`.
    /// - **FinalHop**: delivers the plaintext payload directly to the destination
    ///   client (identified by byte 0 of the destination address).
    fn mix(
        &mut self,
        _: SphinxMessage,
        payload: TimedPayload<Ts>,
        timestamp: Ts,
    ) -> Vec<AddressedTimedPayload<Ts, NodeId>> {
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
                    let timed_sphinx = AddressedTimedData::new(
                        timestamp.add_delay(delay),
                        next_hop_packet.to_bytes(),
                        next_hop_address.as_bytes()[0],
                    );
                    vec![timed_sphinx]
                }
                nym_sphinx::ProcessedPacketData::FinalHop {
                    destination,
                    identifier: _,
                    payload,
                } => {
                    if let Ok(plaintext) = payload
                        .recover_plaintext()
                        .inspect_err(|e| tracing::warn!("Impossible to recover plaintext : {e}"))
                    {
                        let (surb_ack_bytes, message) = SurbAck::extract_ack_and_message(plaintext);
                        let mut packets_to_forward = vec![AddressedTimedData::new(
                            timestamp.clone(),
                            message,
                            destination.as_bytes()[0],
                        )];
                        if !surb_ack_bytes.is_empty()
                            && let Ok((next_hop, surb_ack)) = SurbAck::try_recover_first_hop_packet(
                                &surb_ack_bytes,
                            )
                            .inspect_err(|e| tracing::warn!("Fail to deserialize SURB Ack : {e}"))
                        {
                            packets_to_forward.push(AddressedTimedData::new(
                                timestamp,
                                surb_ack.to_bytes(),
                                next_hop,
                            ));
                        }
                        packets_to_forward
                    } else {
                        Vec::new()
                    }
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
