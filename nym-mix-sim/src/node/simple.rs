// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! [`SimpleNode`] — mix node using the simple (non-Sphinx) packet pipeline.

use std::{net::SocketAddr, sync::Arc, time::Instant};

use nym_lp_data::{
    AddressedTimedData, AddressedTimedPayload, TimedData, TimedPayload,
    common::traits::{
        Framing, FramingUnwrap, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
    nymnodes::traits::NymNodeProcessingPipeline,
};

use crate::{
    node::{BaseNode, NodeId},
    packet::simple::{SimpleFrame, SimplePacket, SimpleWireUnwrapper, SimpleWireWrapper},
    topology::{TopologyNode, directory::Directory},
};

/// A mix-node that uses the simple (non-Sphinx) packet pipeline.
///
/// This is a type alias for [`BaseNode`] specialised to [`SimplePacket`] and
/// [`SimpleProcessingNode`].  All tick logic lives in the generic
/// [`MixSimNode`] impl on `BaseNode`.
///
/// [`MixSimNode`]: crate::node::MixSimNode
pub type SimpleNode = BaseNode<SimplePacket, SimpleFrame, SimpleProcessingNode>;

impl SimpleNode {
    /// Create a [`SimpleNode`] from a [`TopologyNode`] description by binding a
    /// non-blocking UDP socket to `node.socket_address`.
    ///
    /// # Errors
    ///
    /// Returns an error if the UDP socket cannot be bound or set non-blocking.
    pub fn new(topology_node: TopologyNode, directory: Arc<Directory>) -> anyhow::Result<Self> {
        let pipeline = SimpleProcessingNode::new(topology_node.node_id, directory);
        BaseNode::with_pipeline(
            topology_node.node_id,
            topology_node.reliability,
            topology_node.socket_address,
            pipeline,
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// A simple [`NymNodeProcessingPipeline`] for [`SimplePacket`].
///
/// Demonstrates the full pipeline: unwraps the incoming packet through the
/// wire layer (transport → frame → payload), applies a routing decision in
/// [`NymNodeProcessingPipeline::mix`] (forwards to `self.id + 1`), then
/// re-wraps the outgoing payload (payload → frame → transport) before sending.
pub struct SimpleProcessingNode {
    next_hop: SocketAddr,
    wrapper: SimpleWireWrapper,
    unwrapper: SimpleWireUnwrapper,
}

impl SimpleProcessingNode {
    /// Construct a pipeline for the node identified by `id`.
    pub fn new(id: NodeId, directory: Arc<Directory>) -> Self {
        // SAFETY : clients have the highest ID so there will be something ad id+1
        #[allow(clippy::unwrap_used)]
        let next_hop = directory
            .node(id + 1)
            .map(|n| n.addr)
            .or(directory.client(id + 1).map(|c| c.addr))
            .unwrap();
        Self {
            next_hop,
            wrapper: SimpleWireWrapper,
            unwrapper: SimpleWireUnwrapper,
        }
    }
}

impl NymNodeProcessingPipeline<SimpleFrame> for SimpleProcessingNode {
    fn frame_size(&self) -> usize {
        <SimpleWireWrapper as WireWrappingPipeline<SimplePacket, ()>>::frame_size(&self.wrapper)
    }

    type Options = ();
    type MessageKind = ();

    /// Route the payload to the next node in the chain (`self.id + 1`).
    ///
    /// This is a trivial fixed routing rule used for simulation testing.
    fn mix(
        &mut self,
        _: (),
        payload: TimedPayload,
        _timestamp: Instant,
    ) -> Vec<AddressedTimedPayload> {
        vec![AddressedTimedPayload::new_addressed(
            payload.timestamp,
            payload.data,
            self.next_hop,
        )]
    }
}

// Delegation of subtraits
impl Framing<()> for SimpleProcessingNode {
    type Frame = SimpleFrame;
    const OVERHEAD_SIZE: usize = <SimpleWireWrapper as Framing<_>>::OVERHEAD_SIZE;
    fn to_frame(
        &mut self,
        payload: AddressedTimedPayload,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<SimpleFrame>> {
        self.wrapper.to_frame(payload, frame_size)
    }
}

impl Transport<SimplePacket> for SimpleProcessingNode {
    type Frame = SimpleFrame;
    type Error = <SimpleWireWrapper as Transport<SimplePacket>>::Error;
    const OVERHEAD_SIZE: usize = <SimpleWireWrapper as Transport<_>>::OVERHEAD_SIZE;
    fn to_transport_packet(
        &mut self,
        frame: AddressedTimedData<SimpleFrame>,
    ) -> Result<AddressedTimedData<SimplePacket>, Self::Error> {
        self.wrapper.to_transport_packet(frame)
    }
}

impl WireWrappingPipeline<SimplePacket, ()> for SimpleProcessingNode {
    fn packet_size(&self) -> usize {
        <SimpleWireWrapper as WireWrappingPipeline<_, _>>::packet_size(&self.wrapper)
    }
}

impl FramingUnwrap<()> for SimpleProcessingNode {
    type Frame = SimpleFrame;
    fn frame_to_message(&mut self, frame: TimedData<SimpleFrame>) -> Option<(TimedPayload, ())> {
        self.unwrapper.frame_to_message(frame)
    }
}

impl TransportUnwrap<SimplePacket> for SimpleProcessingNode {
    type Frame = SimpleFrame;
    type Error = anyhow::Error;
    fn packet_to_frame(
        &mut self,
        packet: SimplePacket,
        timestamp: Instant,
    ) -> anyhow::Result<TimedData<SimpleFrame>> {
        self.unwrapper.packet_to_frame(packet, timestamp)
    }
}

impl WireUnwrappingPipeline<SimplePacket, ()> for SimpleProcessingNode {}
