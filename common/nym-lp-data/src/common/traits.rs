// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use std::time::Instant;

use crate::{AddressedTimedData, PipelinePayload, TimedData, TimedPayload};

/// Trait for applying framing to a timed payload.
///
/// # Type Parameters
/// - `Opts` : Opts type carried by the `PipelinePayload`
/// - `NdId` : how the destination is named at this stage. Defaults to a [`SocketAddr`], which is
///   right wherever routing has already picked a wire address. A pipeline that must keep the
///   *identity* of the destination - because that is what selects the encryption - names it here
///   instead, and [`Transport`] resolves it to a wire address.
///
/// # Associated Types
/// - `Frame`: Frame type produced by the framing operation.
///
/// # Associated Constants
/// - `OVERHEAD_SIZE`: Number of additional bytes added by the framing scheme.
///
/// # Required Methods
/// - `to_frame`: Splits the payload into a `Vec<AddressedTimedData<Self::Frame>>` of frames of the given size.
pub trait Framing<Opts, NdId = SocketAddr> {
    type Frame;
    const OVERHEAD_SIZE: usize;
    fn to_frame(
        &mut self,
        payload: PipelinePayload<Opts, NdId>,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<Self::Frame, NdId>>;
}

/// Trait for unwrapping framing from a frame back into a payload.
///
/// # Type Parameters
/// - `Mk`: Enum describing the kind of message that can be returned.
///
/// # Associated Types
/// - `Frame`: Frame type consumed as input.
///
/// # Required Methods
/// - `frame_to_message`: Attempts to reassemble a payload from the given frame, returning
///   `Some((payload, kind))` when a complete message is available, or `None` otherwise.
pub trait FramingUnwrap<Mk> {
    type Frame;
    fn frame_to_message(&mut self, frame: TimedData<Self::Frame>) -> Option<(TimedPayload, Mk)>;
}

/// Trait for applying a transport layer to a framed payload.
///
/// # Type Parameters
/// - `Pkt`: Transport packet type produced as output.
///
/// # Associated Types
/// - `Frame`: Frame type consumed as input.
/// - `Error`: Error type
///
/// # Associated Constants
/// - `OVERHEAD_SIZE`: Number of additional bytes added by the transport scheme.
///
/// # Required Methods
/// - `to_transport_packet`: Wraps a frame into a transport packet.
///
/// Fallible, mirroring [`TransportUnwrap`]: an encrypting transport has to resolve a session for
/// the frame's destination, and may not have one. Implementations that cannot fail use
/// [`Infallible`](std::convert::Infallible).
/// The frame arrives addressed by `NdId` and leaves addressed by a [`SocketAddr`]: resolving the
/// destination's identity to somewhere on the wire is part of wrapping it, because both answers
/// come from the same place - the session store that knows who a peer is and where it was last
/// seen.
pub trait Transport<Pkt, NdId = SocketAddr> {
    type Frame;
    type Error;
    const OVERHEAD_SIZE: usize;
    fn to_transport_packet(
        &mut self,
        frame: AddressedTimedData<Self::Frame, NdId>,
    ) -> Result<AddressedTimedData<Pkt>, Self::Error>;
}

/// Trait for unwrapping a transport packet back into a frame.
///
/// # Type Parameters
/// - `Pkt`: Transport packet type consumed as input.
///
/// # Associated Types
/// - `Frame`: Frame type produced as output.
/// - `Error`: Error type
///
/// # Required Methods
/// - `packet_to_frame`: Strips the transport layer from a packet, returning the inner frame
///   tagged with the given timestamp.
pub trait TransportUnwrap<Pkt> {
    type Frame;
    type Error;
    fn packet_to_frame(
        &mut self,
        packet: Pkt,
        timestamp: Instant,
    ) -> Result<TimedData<Self::Frame>, Self::Error>;
}

/// Supertrait combining [`Framing`] and [`Transport`] into a reusable wire-wrapping layer.
///
/// Used as the bottom stage of any outbound pipeline (client or mixnode).
///
/// # Type Parameters
/// - `Pkt`: Final transport packet type.
/// - `Opts` : Option type
///
/// Both [`Framing`] and [`Transport`] declare their own `type Frame`; this
/// supertrait cross-constrains them so `to_frame`'s output feeds directly into
/// `to_transport_packet`.
///
/// # Required Methods
/// - `packet_size`: Total on-wire size of an output packet in bytes.
///
/// # Provided Methods
/// - `frame_size`: Derived from `packet_size` minus transport and framing overheads.
/// - `wire_wrap`: Frames a payload and wraps each frame into a transport packet.
pub trait WireWrappingPipeline<Pkt, Opts, NdId = SocketAddr>:
    Transport<Pkt, NdId> + Framing<Opts, NdId, Frame = <Self as Transport<Pkt, NdId>>::Frame>
{
    // IMPORTANT NOTE : This fn can be not constant to allow e.g. flexible MTU
    // However, every possible value must be able to accommodate the different overhead.
    // If it doesn't, the pipeline becomes unusable
    fn packet_size(&self) -> usize;

    fn frame_size(&self) -> usize {
        // SAFETY : While this CAN technically fail, it means that something is wrong in the code and it's pointless to continue anyway
        #[allow(clippy::expect_used)]
        self.packet_size()
            .checked_sub(
                <Self as Transport<Pkt, NdId>>::OVERHEAD_SIZE
                    + <Self as Framing<Opts, NdId>>::OVERHEAD_SIZE,
            )
            .expect("packet_size smaller than transport + framing overhead")
    }

    fn wire_wrap(
        &mut self,
        payload: PipelinePayload<Opts, NdId>,
    ) -> Result<Vec<AddressedTimedData<Pkt>>, <Self as Transport<Pkt, NdId>>::Error> {
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
pub trait WireUnwrappingPipeline<Pkt, Mk>:
    TransportUnwrap<Pkt> + FramingUnwrap<Mk, Frame = <Self as TransportUnwrap<Pkt>>::Frame>
{
    fn wire_unwrap(
        &mut self,
        input: Pkt,
        timestamp: Instant,
    ) -> Result<Option<(TimedPayload, Mk)>, Self::Error> {
        let frame = self.packet_to_frame(input, timestamp)?;
        Ok(self.frame_to_message(frame))
    }
}
