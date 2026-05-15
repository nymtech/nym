// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! [`SimpleNode`] — mix node using the simple (non-Sphinx) packet pipeline.

use std::sync::Arc;

use nym_lp_data::{
    AddressedTimedData, AddressedTimedPayload, TimedData, TimedPayload,
    common::traits::{
        Framing, FramingUnwrap, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
    mixnodes::traits::MixnodeProcessingPipeline,
};

use crate::{
    node::{BaseNode, NodeId, ProcessingNode},
    packet::simple::{
        SimpleFrame, SimpleMessage, SimplePacket, SimpleWireUnwrapper, SimpleWireWrapper,
    },
    topology::{TopologyNode, directory::Directory},
};

/// A mix-node that uses the simple (non-Sphinx) packet pipeline.
///
/// This is a type alias for [`BaseNode`] specialised to [`SimplePacket`] and
/// [`SimpleProcessingNode`].  All tick logic lives in the generic
/// [`MixSimNode`] impl on `BaseNode`.
///
/// [`MixSimNode`]: crate::node::MixSimNode
pub type SimpleNode<Ts> = BaseNode<Ts, SimplePacket, SimpleProcessingNode>;

impl<Ts> SimpleNode<Ts> {
    /// Create a [`SimpleNode`] from a [`TopologyNode`] description by binding a
    /// non-blocking UDP socket to `node.socket_address`.
    ///
    /// # Errors
    ///
    /// Returns an error if the UDP socket cannot be bound or set non-blocking.
    pub fn new(topology_node: TopologyNode, directory: Arc<Directory>) -> anyhow::Result<Self> {
        let pipeline = SimpleProcessingNode::new(topology_node.node_id);
        BaseNode::with_pipeline(
            topology_node.node_id,
            topology_node.reliability,
            topology_node.socket_address,
            directory,
            pipeline,
        )
    }
}

impl<Ts: Clone> ProcessingNode<Ts, SimplePacket> for SimpleProcessingNode {
    fn process(
        &mut self,
        input: TimedData<Ts, SimplePacket>,
        timestamp: Ts,
    ) -> anyhow::Result<Vec<AddressedTimedData<Ts, SimplePacket, NodeId>>> {
        MixnodeProcessingPipeline::<Ts, SimplePacket, SimpleMessage, NodeId>::process(
            self, input, timestamp,
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// A simple [`MixnodeProcessingPipeline`] for [`SimplePacket`].
///
/// Demonstrates the full pipeline: unwraps the incoming packet through the
/// wire layer (transport → frame → payload), applies a routing decision in
/// [`mix`] (forwards to `self.id + 1`), then re-wraps the outgoing payload
/// (payload → frame → transport) before sending.
pub struct SimpleProcessingNode {
    id: NodeId,
    wrapper: SimpleWireWrapper,
    unwrapper: SimpleWireUnwrapper,
}

impl SimpleProcessingNode {
    /// Construct a pipeline for the node identified by `id`.
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            wrapper: SimpleWireWrapper,
            unwrapper: SimpleWireUnwrapper,
        }
    }
}

impl<Ts: Clone> MixnodeProcessingPipeline<Ts, SimplePacket, SimpleMessage, NodeId>
    for SimpleProcessingNode
{
    /// Route the payload to the next node in the chain (`self.id + 1`).
    ///
    /// This is a trivial fixed routing rule used for simulation testing.
    /// Real mix nodes would perform cryptographic route unwrapping here.
    fn mix(
        &mut self,
        _: SimpleMessage,
        payload: TimedPayload<Ts>,
        _timestamp: Ts,
    ) -> Vec<AddressedTimedPayload<Ts, NodeId>> {
        vec![(self.id + 1, payload).into()]
    }
}

// Delegation of subtraits
impl<Ts: Clone> Framing<Ts, NodeId> for SimpleProcessingNode {
    type Frame = SimpleFrame;
    const OVERHEAD_SIZE: usize = <SimpleWireWrapper as Framing<Ts, _>>::OVERHEAD_SIZE;
    fn to_frame(
        &self,
        payload: AddressedTimedPayload<Ts, NodeId>,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<Ts, SimpleFrame, NodeId>> {
        self.wrapper.to_frame(payload, frame_size)
    }
}

impl<Ts: Clone> Transport<Ts, SimplePacket, NodeId> for SimpleProcessingNode {
    type Frame = SimpleFrame;
    const OVERHEAD_SIZE: usize = <SimpleWireWrapper as Transport<Ts, _, _>>::OVERHEAD_SIZE;
    fn to_transport_packet(
        &self,
        frame: AddressedTimedData<Ts, SimpleFrame, NodeId>,
    ) -> AddressedTimedData<Ts, SimplePacket, NodeId> {
        self.wrapper.to_transport_packet(frame)
    }
}

impl<Ts: Clone> WireWrappingPipeline<Ts, SimplePacket, NodeId> for SimpleProcessingNode {
    fn packet_size(&self) -> usize {
        <SimpleWireWrapper as WireWrappingPipeline<Ts, _, _>>::packet_size(&self.wrapper)
    }
}

impl<Ts> FramingUnwrap<Ts, SimpleMessage> for SimpleProcessingNode {
    type Frame = SimpleFrame;
    fn frame_to_message(
        &mut self,
        frame: TimedData<Ts, SimpleFrame>,
    ) -> Option<(TimedPayload<Ts>, SimpleMessage)> {
        self.unwrapper.frame_to_message(frame)
    }
}

impl<Ts: Clone> TransportUnwrap<Ts, SimplePacket> for SimpleProcessingNode {
    type Frame = SimpleFrame;
    fn packet_to_frame(
        &self,
        packet: SimplePacket,
        timestamp: Ts,
    ) -> anyhow::Result<TimedData<Ts, SimpleFrame>> {
        self.unwrapper.packet_to_frame(packet, timestamp)
    }
}

impl<Ts: Clone> WireUnwrappingPipeline<Ts, SimplePacket, SimpleMessage> for SimpleProcessingNode {}
