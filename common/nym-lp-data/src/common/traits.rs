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
        &self,
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
    fn packet_to_frame(&self, packet: Pkt, timestamp: Ts) -> TimedData<Ts, Fr>;
}
