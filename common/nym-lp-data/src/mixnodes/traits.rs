// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{TimedData, TimedPayload};

use crate::common::traits::{Framing, Transport};

/// Trait for applying routing security processing (e.g. encryption) to a timed payload.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
///
/// # Required Methods
/// - `process_routing_security`: Process the routing security mechanism to the given payload,
///   returning a new `TimedPayload` with the processed data.
pub trait RoutingSecurityProcessing<Ts> {
    fn process_routing_security(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts>;
}

/// Trait for a mixnode processing pipeline.
///
/// Combines [`Framing`] and [`Transport`] into a single `process` step that takes a
/// `TimedPayload` and returns a list of transport packets ready for sending.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
/// - `Fr`: Frame type produced by the framing layer.
/// - `Pkt`: Transport packet type produced as output.
///
/// # Required Methods
/// - `packet_size`: Size in bytes of the outputted transport packets.
///
/// # Provided Methods
/// - `frame_size`: Derived from `packet_size` minus the transport and framing overheads.
/// - `process`: Frames the payload and wraps each frame into a transport packet.
pub trait ProcessingPipeline<Ts, Fr, Pkt>: Framing<Ts, Fr> + Transport<Ts, Fr, Pkt>
where
    Ts: Clone,
{
    fn packet_size(&self) -> usize;
    fn frame_size(&self) -> usize {
        self.packet_size()
            - <Self as Transport<_, _, _>>::OVERHEAD_SIZE
            - <Self as Framing<_, _>>::OVERHEAD_SIZE
    }

    fn process(&mut self, input: TimedPayload<Ts>) -> Vec<TimedData<Ts, Pkt>> {
        let frames = self.to_frame(input, self.frame_size());
        frames
            .into_iter()
            .map(|frame| self.to_transport_packet(frame))
            .collect::<Vec<_>>()
    }
}

/// Top-level processing trait for a mix node.
///
/// Takes a received transport packet, applies the full mix operation
/// (unwrap, route, re-wrap), and returns zero or more `(next_hop, packet)`
/// pairs indicating where each output packet should be forwarded.
///
/// # Type Parameters
/// - `Ts`: Timestamp / tick-context type.
/// - `Pkt`: Transport packet type; the same type is consumed and produced.
/// - `NodeId`: Identifier type for the next-hop destination.
pub trait MixnodeProcessingPipeline<Ts, Pkt, NodeId> {
    fn process(
        &mut self,
        input: TimedData<Ts, Pkt>,
        timestamp: Ts,
    ) -> Vec<(NodeId, TimedData<Ts, Pkt>)>;
}
