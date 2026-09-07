// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::convert::Infallible;
use std::time::Instant;

use crate::{
    AddressedTimedData, AddressedTimedPayload, PipelinePayload, TimedData, TimedPayload,
    common::traits::{
        Framing, FramingUnwrap, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
};

/// Marker trait for a no-op [`WireWrappingPipeline`] implementation.
///
/// Implement this for your pipeline type to get a [`WireWrappingPipeline`] impl that
/// passes the payload through unchanged with zero byte overhead.
pub trait NoOpWireWrapper {
    const PACKET_SIZE: usize = 1500;
}

impl<T, Opts> Framing<Opts> for T
where
    T: NoOpWireWrapper,
{
    type Frame = Vec<u8>;
    const OVERHEAD_SIZE: usize = 0;
    fn to_frame(&mut self, payload: PipelinePayload<Opts>, _: usize) -> Vec<AddressedTimedPayload> {
        vec![payload.into_addressed()]
    }
}

impl<T, Pkt> Transport<Pkt> for T
where
    T: NoOpWireWrapper,
    Pkt: From<Vec<u8>>,
{
    type Frame = Vec<u8>;
    type Error = Infallible;
    const OVERHEAD_SIZE: usize = 0;
    fn to_transport_packet(
        &mut self,
        frame: AddressedTimedPayload,
    ) -> Result<AddressedTimedData<Pkt>, Self::Error> {
        Ok(frame.data_transform(|data| data.into()))
    }
}

impl<T, Pkt, Opts> WireWrappingPipeline<Pkt, Opts> for T
where
    T: NoOpWireWrapper,
    Pkt: From<Vec<u8>>,
{
    fn packet_size(&self) -> usize {
        T::PACKET_SIZE
    }
}

/// Marker trait for a no-op [`WireUnwrappingPipeline`] implementation.
///
/// Implement this for your pipeline type to get a [`WireUnwrappingPipeline`] impl that
/// passes the payload through unchanged.
pub trait NoOpWireUnwrapper {}

impl<T, Mk> FramingUnwrap<Mk> for T
where
    T: NoOpWireUnwrapper,
    Mk: Default,
{
    type Frame = Vec<u8>;
    fn frame_to_message(&mut self, frame: TimedPayload) -> Option<(TimedPayload, Mk)> {
        Some((frame, Default::default()))
    }
}

impl<T, Pkt> TransportUnwrap<Pkt> for T
where
    T: NoOpWireUnwrapper,
    Pkt: Into<Vec<u8>>,
{
    type Frame = Vec<u8>;
    type Error = std::convert::Infallible;
    fn packet_to_frame(
        &mut self,
        packet: Pkt,
        timestamp: Instant,
    ) -> Result<TimedPayload, Self::Error> {
        Ok(TimedData {
            timestamp,
            data: packet.into(),
        })
    }
}

impl<T, Pkt, Mk> WireUnwrappingPipeline<Pkt, Mk> for T
where
    T: NoOpWireUnwrapper,
    Pkt: Into<Vec<u8>>,
    Mk: Default,
{
}
