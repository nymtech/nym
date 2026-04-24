// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use nym_crypto::asymmetric::x25519;
use nym_lp_data::{
    TimedData, TimedPayload,
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
    ) -> anyhow::Result<Vec<(NodeId, TimedData<Ts, SimSphinxPacket>)>> {
        MixnodeProcessingPipeline::<Ts, SphinxPacket, SimSphinxPacket, SphinxMessage, NodeId>::process(
            self, input, timestamp,
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// A [`MixnodeProcessingPipeline`] for [`SphinxPacket`], with no framing and no transport layer
///
pub struct SphinxProcessingNode {
    id: NodeId,
    sphinx_secret: x25519::PrivateKey,
    wrapper: SphinxNoOpWireWrapper,
    unwrapper: SphinxNoOpWireUnwrapper,
}

impl SphinxProcessingNode {
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
    fn mix(
        &mut self,
        _: SphinxMessage,
        payload: TimedPayload<Ts>,
        timestamp: Ts,
    ) -> Vec<(NodeId, TimedPayload<Ts>)> {
        // SAFETY : Given the unwrapper we are using, it is guaranteed to be a sphinx packet here
        #[allow(clippy::unwrap_used)]
        let sphinx_packet = SphinxPacket::from_bytes(&payload.data).unwrap();

        match sphinx_packet.process(self.sphinx_secret.inner()) {
            Ok(packet) => match packet.data {
                nym_sphinx::ProcessedPacketData::ForwardHop {
                    next_hop_packet,
                    next_hop_address,
                    delay,
                } => {
                    let timed_sphinx = TimedData {
                        timestamp: timestamp.add_delay(delay),
                        data: next_hop_packet.to_bytes(),
                    };
                    vec![(next_hop_address.as_bytes()[0], timed_sphinx)]
                }
                nym_sphinx::ProcessedPacketData::FinalHop {
                    destination,
                    identifier: _,
                    payload,
                } => {
                    vec![(
                        destination.as_bytes()[0],
                        TimedData {
                            data: payload.into_bytes(),
                            timestamp,
                        },
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
impl<Ts: Clone> Framing<Ts, SphinxPacket> for SphinxProcessingNode {
    const OVERHEAD_SIZE: usize = <SphinxNoOpWireWrapper as Framing<Ts, _>>::OVERHEAD_SIZE;
    fn to_frame(
        &self,
        payload: TimedPayload<Ts>,
        frame_size: usize,
    ) -> Vec<TimedData<Ts, SphinxPacket>> {
        self.wrapper.to_frame(payload, frame_size)
    }
}

impl<Ts: Clone> Transport<Ts, SphinxPacket, SimSphinxPacket> for SphinxProcessingNode {
    const OVERHEAD_SIZE: usize = <SphinxNoOpWireWrapper as Transport<Ts, _, _>>::OVERHEAD_SIZE;
    fn to_transport_packet(
        &self,
        frame: TimedData<Ts, SphinxPacket>,
    ) -> TimedData<Ts, SimSphinxPacket> {
        self.wrapper.to_transport_packet(frame)
    }
}

impl<Ts: Clone> WireWrappingPipeline<Ts, SphinxPacket, SimSphinxPacket> for SphinxProcessingNode {
    fn packet_size(&self) -> usize {
        <SphinxNoOpWireWrapper as WireWrappingPipeline<Ts, _, _>>::packet_size(&self.wrapper)
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
