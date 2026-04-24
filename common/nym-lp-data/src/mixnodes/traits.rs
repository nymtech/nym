// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{TimedData, TimedPayload};

use crate::common::traits::{Framing, FramingUnwrap, Transport, TransportUnwrap};

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

/// Trait for a mixnode unwrapping pipeline.
///
/// Combines [`TransportUnwrap`] and [`FramingUnwrap`] into a single `process` step that
/// takes a transport packet and returns a reassembled payload with its message kind, if
/// the packet completes a message.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
/// - `Fr`: Frame type produced by the transport layer.
/// - `Pkt`: Transport packet type consumed as input.
pub trait UnwrappingPipeline<Ts, Fr, Pkt>:
    TransportUnwrap<Ts, Fr, Pkt> + FramingUnwrap<Ts, Fr>
where
    Ts: Clone,
{
    fn process(
        &mut self,
        input: Pkt,
        timestamp: Ts,
    ) -> Option<(TimedPayload<Ts>, Self::MessageKind)> {
        let frame = self.packet_to_frame(input, timestamp.clone());

        self.frame_to_message(frame)
    }
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
