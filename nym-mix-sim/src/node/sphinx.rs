// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! [`SphinxNode`] — mix node using the full Sphinx packet pipeline.

use std::sync::Arc;

use nym_crypto::asymmetric::x25519;
use nym_lp_data::{
    AddressedTimedData, AddressedTimedPayload, TimedData, TimedPayload,
    common::traits::{
        Framing, FramingUnwrap, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
    mixnodes::traits::MixnodeProcessingPipeline,
};
use nym_sphinx::SphinxPacket;

use crate::{
    node::{BaseNode, NodeId, ProcessingNode},
    packet::sphinx::{
        AddDelay, SimSphinxPacket, SphinxMessage, SphinxNoOpWireUnwrapper, SphinxNoOpWireWrapper,
    },
    topology::{TopologyNode, directory::Directory},
};

/// A mix-node that uses the Sphinx packet pipeline.
///
/// This is a type alias for [`BaseNode`] specialised to [`SphinxPacket`] and
/// [`SphinxMixnodePipeline`].  All tick logic lives in the generic
/// [`MixSimNode`] impl on `BaseNode`.
///
/// [`MixSimNode`]: crate::node::MixSimNode
pub type SphinxNode<Ts> = BaseNode<Ts, SimSphinxPacket, SphinxProcessingNode>;

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

impl<Ts> ProcessingNode<Ts, SimSphinxPacket> for SphinxProcessingNode
where
    Ts: AddDelay + Clone,
{
    fn process(
        &mut self,
        input: TimedData<Ts, SimSphinxPacket>,
        timestamp: Ts,
    ) -> anyhow::Result<Vec<AddressedTimedData<Ts, SimSphinxPacket, NodeId>>> {
        MixnodeProcessingPipeline::<Ts, SphinxPacket, SimSphinxPacket, SphinxMessage, NodeId>::process(
            self, input, timestamp,
        )
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
    wrapper: SphinxNoOpWireWrapper,
    unwrapper: SphinxNoOpWireUnwrapper,
}

impl SphinxProcessingNode {
    /// Construct a pipeline for the node identified by `node_id`, using
    /// `sphinx_secret` to decrypt incoming Sphinx packets.
    pub fn new(node_id: NodeId, sphinx_secret: x25519::PrivateKey) -> Self {
        Self {
            id: node_id,
            sphinx_secret,
            wrapper: SphinxNoOpWireWrapper,
            unwrapper: SphinxNoOpWireUnwrapper,
        }
    }
}

impl<Ts> MixnodeProcessingPipeline<Ts, SphinxPacket, SimSphinxPacket, SphinxMessage, NodeId>
    for SphinxProcessingNode
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
                    vec![AddressedTimedData::new(
                        timestamp,
                        payload.into_bytes(),
                        destination.as_bytes()[0],
                    )]
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
impl<Ts: Clone> Framing<Ts, SphinxPacket, NodeId> for SphinxProcessingNode {
    const OVERHEAD_SIZE: usize = <SphinxNoOpWireWrapper as Framing<Ts, _, _>>::OVERHEAD_SIZE;
    fn to_frame(
        &self,
        payload: AddressedTimedPayload<Ts, NodeId>,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<Ts, SphinxPacket, NodeId>> {
        self.wrapper.to_frame(payload, frame_size)
    }
}

impl<Ts: Clone> Transport<Ts, SphinxPacket, SimSphinxPacket, NodeId> for SphinxProcessingNode {
    const OVERHEAD_SIZE: usize = <SphinxNoOpWireWrapper as Transport<Ts, _, _, _>>::OVERHEAD_SIZE;
    fn to_transport_packet(
        &self,
        frame: AddressedTimedData<Ts, SphinxPacket, NodeId>,
    ) -> AddressedTimedData<Ts, SimSphinxPacket, NodeId> {
        self.wrapper.to_transport_packet(frame)
    }
}

impl<Ts: Clone> WireWrappingPipeline<Ts, SphinxPacket, SimSphinxPacket, NodeId>
    for SphinxProcessingNode
{
    fn packet_size(&self) -> usize {
        <SphinxNoOpWireWrapper as WireWrappingPipeline<Ts, _, _, _>>::packet_size(&self.wrapper)
    }
}

impl<Ts> FramingUnwrap<Ts, SphinxPacket, SphinxMessage> for SphinxProcessingNode {
    fn frame_to_message(
        &mut self,
        frame: TimedData<Ts, SphinxPacket>,
    ) -> Option<(TimedPayload<Ts>, SphinxMessage)> {
        self.unwrapper.frame_to_message(frame)
    }
}

impl<Ts: Clone> TransportUnwrap<Ts, SphinxPacket, SimSphinxPacket> for SphinxProcessingNode {
    fn packet_to_frame(
        &self,
        packet: SimSphinxPacket,
        timestamp: Ts,
    ) -> anyhow::Result<TimedData<Ts, SphinxPacket>> {
        self.unwrapper.packet_to_frame(packet, timestamp)
    }
}

impl<Ts: Clone> WireUnwrappingPipeline<Ts, SphinxPacket, SimSphinxPacket, SphinxMessage>
    for SphinxProcessingNode
{
}
