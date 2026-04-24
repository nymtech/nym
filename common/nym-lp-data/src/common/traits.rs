// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{AddressedTimedData, AddressedTimedPayload, TimedData, TimedPayload};

/// Trait for applying framing to a timed payload.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
///
/// # Associated Types
/// - `Frame`: Frame type produced by the framing operation.
///
/// # Associated Constants
/// - `OVERHEAD_SIZE`: Number of additional bytes added by the framing scheme.
///
/// # Required Methods
/// - `to_frame`: Splits the payload into a `Vec<TimedData<Ts, Self::Frame>>` of frames of the given size.
pub trait Framing<Ts, NdId> {
    type Frame;
    const OVERHEAD_SIZE: usize;
    fn to_frame(
        &self,
        payload: AddressedTimedPayload<Ts, NdId>,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<Ts, Self::Frame, NdId>>;
}

/// Trait for unwrapping framing from a frame back into a payload.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
/// - `Mk`: Enum describing the kind of message that can be returned.
///
/// # Associated Types
/// - `Frame`: Frame type consumed as input.
///
/// # Required Methods
/// - `frame_to_message`: Attempts to reassemble a payload from the given frame, returning
///   `Some((payload, kind))` when a complete message is available, or `None` otherwise.
pub trait FramingUnwrap<Ts, Mk> {
    type Frame;
    fn frame_to_message(
        &mut self,
        frame: TimedData<Ts, Self::Frame>,
    ) -> Option<(TimedPayload<Ts>, Mk)>;
}

/// Trait for applying a transport layer to a framed payload.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
/// - `Pkt`: Transport packet type produced as output.
///
/// # Associated Constants
/// - `OVERHEAD_SIZE`: Number of additional bytes added by the transport scheme.
///
/// # Required Methods
/// - `to_transport_packet`: Wraps a frame into a transport packet. The frame type is
///   inherited from the [`Framing`] supertrait via `Self::Frame`.
pub trait Transport<Ts, Pkt, NdId>: Framing<Ts, NdId> {
    const OVERHEAD_SIZE: usize;
    fn to_transport_packet(
        &self,
        frame: AddressedTimedData<Ts, Self::Frame, NdId>,
    ) -> AddressedTimedData<Ts, Pkt, NdId>;
}

/// Trait for unwrapping a transport packet back into a frame.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
/// - `Pkt`: Transport packet type consumed as input.
///
/// # Associated Types
/// - `Frame`: Frame type produced as output.
///
/// # Required Methods
/// - `packet_to_frame`: Strips the transport layer from a packet, returning the inner frame
///   tagged with the given timestamp.
pub trait TransportUnwrap<Ts, Pkt> {
    type Frame;
    fn packet_to_frame(
        &self,
        packet: Pkt,
        timestamp: Ts,
    ) -> anyhow::Result<TimedData<Ts, Self::Frame>>;
}

/// Supertrait combining [`Framing`] and [`Transport`] into a reusable wire-wrapping layer.
///
/// Used as the bottom stage of any outbound pipeline (client or mixnode).
///
/// # Type Parameters
/// - `Ts`: Timestamp type.
/// - `Pkt`: Final transport packet type.
///
/// # Required Methods
/// - `packet_size`: Total on-wire size of an output packet in bytes.
///
/// # Provided Methods
/// - `frame_size`: Derived from `packet_size` minus transport and framing overheads.
/// - `wire_wrap`: Frames a payload and wraps each frame into a transport packet.
pub trait WireWrappingPipeline<Ts, Pkt, NdId>:
    Framing<Ts, NdId> + Transport<Ts, Pkt, NdId>
where
    Ts: Clone,
    NdId: Clone,
{
    fn packet_size(&self) -> usize;

    fn frame_size(&self) -> usize {
        self.packet_size()
            - <Self as Transport<Ts, Pkt, NdId>>::OVERHEAD_SIZE
            - <Self as Framing<Ts, NdId>>::OVERHEAD_SIZE
    }

    fn wire_wrap(
        &self,
        payload: AddressedTimedPayload<Ts, NdId>,
    ) -> Vec<AddressedTimedData<Ts, Pkt, NdId>> {
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
/// - `Pkt`: Transport packet type consumed as input.
/// - `Mk`: Message-kind marker returned alongside the reassembled payload.
///
/// Both [`TransportUnwrap`] and [`FramingUnwrap`] declare their own `type Frame`;
/// this supertrait cross-constrains them so `packet_to_frame`'s output feeds
/// directly into `frame_to_message`.
///
/// # Provided Methods
/// - `wire_unwrap`: Strips the transport layer from a packet and attempts to reassemble
///   a payload, returning `Some((payload, kind))` when a complete message is available.
pub trait WireUnwrappingPipeline<Ts, Pkt, Mk>:
    TransportUnwrap<Ts, Pkt>
    + FramingUnwrap<Ts, Mk, Frame = <Self as TransportUnwrap<Ts, Pkt>>::Frame>
where
    Ts: Clone,
{
    fn wire_unwrap(
        &mut self,
        input: Pkt,
        timestamp: Ts,
    ) -> anyhow::Result<Option<(TimedPayload<Ts>, Mk)>> {
        let frame = self.packet_to_frame(input, timestamp)?;
        Ok(self.frame_to_message(frame))
    }
}
