// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{TimedData, TimedPayload};

/// Trait for applying framing to a timed payload.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
/// - `Fr`: Frame type produced by the framing operation.
///
/// # Associated Constants
/// - `OVERHEAD_SIZE`: Number of additional bytes added by the framing scheme.
///
/// # Required Methods
/// - `to_frame`: Splits the payload into a `Vec<TimedData<Ts, Fr>>` of frames of the given size.
pub trait Framing<Ts, Fr> {
    const OVERHEAD_SIZE: usize;
    fn to_frame(&self, payload: TimedPayload<Ts>, frame_size: usize) -> Vec<TimedData<Ts, Fr>>;
}

/// Trait for unwrapping framing from a frame back into a payload.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
/// - `Fr`: Frame type consumed as input.
///
/// # Associated Types
/// - `MessageKind`: Enum describing the kind of message that can be returned.
///
/// # Required Methods
/// - `frame_to_message`: Attempts to reassemble a payload from the given frame, returning
///   `Some((payload, kind))` when a complete message is available, or `None` otherwise.
pub trait FramingUnwrap<Ts, Fr> {
    // The enum describing the kind of message that can be returned
    type MessageKind;
    fn frame_to_message(
        &mut self,
        frame: TimedData<Ts, Fr>,
    ) -> Option<(TimedPayload<Ts>, Self::MessageKind)>;
}

/// Trait for applying a transport layer to a framed payload.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
/// - `Fr`: Frame type consumed as input.
/// - `Pkt`: Transport packet type produced as output.
///
/// # Associated Constants
/// - `OVERHEAD_SIZE`: Number of additional bytes added by the transport scheme.
///
/// # Required Methods
/// - `to_transport_packet`: Wraps a frame into a transport packet.
pub trait Transport<Ts, Fr, Pkt> {
    const OVERHEAD_SIZE: usize;
    fn to_transport_packet(&self, frame: TimedData<Ts, Fr>) -> TimedData<Ts, Pkt>;
}

/// Trait for unwrapping a transport packet back into a frame.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
/// - `Fr`: Frame type produced as output.
/// - `Pkt`: Transport packet type consumed as input.
///
/// # Required Methods
/// - `packet_to_frame`: Strips the transport layer from a packet, returning the inner frame
///   tagged with the given timestamp.
pub trait TransportUnwrap<Ts, Fr, Pkt> {
    fn packet_to_frame(&self, packet: Pkt, timestamp: Ts) -> anyhow::Result<TimedData<Ts, Fr>>;
}

/// Supertrait combining [`Framing`] and [`Transport`] into a reusable wire-wrapping layer.
///
/// Used as the bottom stage of any outbound pipeline (client or mixnode).
///
/// # Type Parameters
/// - `Ts`: Timestamp type.
/// - `Fr`: Intermediate frame type produced by framing.
/// - `Pkt`: Final transport packet type.
///
/// # Required Methods
/// - `packet_size`: Total on-wire size of an output packet in bytes.
///
/// # Provided Methods
/// - `frame_size`: Derived from `packet_size` minus transport and framing overheads.
/// - `wire_wrap`: Frames a payload and wraps each frame into a transport packet.
pub trait WireWrappingPipeline<Ts, Fr, Pkt>: Framing<Ts, Fr> + Transport<Ts, Fr, Pkt>
where
    Ts: Clone,
{
    fn packet_size(&self) -> usize;

    fn frame_size(&self) -> usize {
        self.packet_size()
            - <Self as Transport<_, _, _>>::OVERHEAD_SIZE
            - <Self as Framing<_, _>>::OVERHEAD_SIZE
    }

    fn wire_wrap(&self, payload: TimedPayload<Ts>) -> Vec<TimedData<Ts, Pkt>> {
        let frame_size = self.frame_size();
        self.to_frame(payload, frame_size)
            .into_iter()
            .map(|frame| self.to_transport_packet(frame))
            .collect()
    }
}

/// Supertrait combining [`TransportUnwrap`] and [`FramingUnwrap`] into a reusable
/// wire-unwrapping layer.
///
/// Used as the bottom stage of any inbound pipeline (client or mixnode).
///
/// # Type Parameters
/// - `Ts`: Timestamp type.
/// - `Fr`: Frame type produced by transport unwrapping.
/// - `Pkt`: Transport packet type consumed as input.
///
/// # Provided Methods
/// - `wire_unwrap`: Strips the transport layer from a packet and attempts to reassemble
///   a payload, returning `Some((payload, kind))` when a complete message is available.
pub trait WireUnwrappingPipeline<Ts, Fr, Pkt>:
    TransportUnwrap<Ts, Fr, Pkt> + FramingUnwrap<Ts, Fr>
where
    Ts: Clone,
{
    fn wire_unwrap(
        &mut self,
        input: Pkt,
        timestamp: Ts,
    ) -> anyhow::Result<Option<(TimedPayload<Ts>, Self::MessageKind)>> {
        let frame = self.packet_to_frame(input, timestamp)?;
        Ok(self.frame_to_message(frame))
    }
}
