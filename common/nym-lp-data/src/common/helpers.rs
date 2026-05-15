// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    AddressedTimedData, AddressedTimedPayload, TimedData, TimedPayload,
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

impl<T, Ts, NdId> Framing<Ts, NdId> for T
where
    T: NoOpWireWrapper,
{
    type Frame = Vec<u8>;
    const OVERHEAD_SIZE: usize = 0;
    fn to_frame(
        &self,
        payload: AddressedTimedPayload<Ts, NdId>,
        _: usize,
    ) -> Vec<AddressedTimedPayload<Ts, NdId>> {
        vec![payload]
    }
}

impl<T, Ts, Pkt, NdId> Transport<Ts, Pkt, NdId> for T
where
    T: NoOpWireWrapper,
    Pkt: From<Vec<u8>>,
{
    type Frame = Vec<u8>;
    const OVERHEAD_SIZE: usize = 0;
    fn to_transport_packet(
        &self,
        frame: AddressedTimedPayload<Ts, NdId>,
    ) -> AddressedTimedData<Ts, Pkt, NdId> {
        frame.data_transform(|data| data.into())
    }
}

impl<T, Ts, Pkt, NdId> WireWrappingPipeline<Ts, Pkt, NdId> for T
where
    T: NoOpWireWrapper,
    Ts: Clone,
    Pkt: From<Vec<u8>>,
    NdId: Clone,
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

impl<T, Ts, Mk> FramingUnwrap<Ts, Mk> for T
where
    T: NoOpWireUnwrapper,
    Mk: Default,
{
    type Frame = Vec<u8>;
    fn frame_to_message(&mut self, frame: TimedPayload<Ts>) -> Option<(TimedPayload<Ts>, Mk)> {
        Some((frame, Default::default()))
    }
}

impl<T, Ts, Pkt> TransportUnwrap<Ts, Pkt> for T
where
    T: NoOpWireUnwrapper,
    Pkt: Into<Vec<u8>>,
{
    type Frame = Vec<u8>;
    type Error = std::convert::Infallible;
    fn packet_to_frame(&self, packet: Pkt, timestamp: Ts) -> Result<TimedPayload<Ts>, Self::Error> {
        Ok(TimedData {
            timestamp,
            data: packet.into(),
        })
    }
}

impl<T, Ts, Pkt, Mk> WireUnwrappingPipeline<Ts, Pkt, Mk> for T
where
    T: NoOpWireUnwrapper,
    Ts: Clone,
    Pkt: Into<Vec<u8>>,
    Mk: Default,
{
}
