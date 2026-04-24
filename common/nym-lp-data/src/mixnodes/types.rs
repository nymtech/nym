// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::common::traits::{Framing, FramingUnwrap, Transport, TransportUnwrap};
use crate::mixnodes::traits::{ProcessingPipeline, UnwrappingPipeline};
use crate::{TimedData, TimedPayload};

/// The generic pipeline struct for a mixnode
pub struct Pipeline<Tu, Fu, F, T> {
    pub packet_size: usize,
    pub transport_unwrap: Tu,
    pub frame_unwrap: Fu,
    pub framing: F,
    pub transport: T,
}

impl<Ts, Fr, Pkt, Tu, Fu, F, T> TransportUnwrap<Ts, Fr, Pkt> for Pipeline<Tu, Fu, F, T>
where
    Tu: TransportUnwrap<Ts, Fr, Pkt>,
{
    fn packet_to_frame(&self, packet: Pkt, timestamp: Ts) -> crate::TimedData<Ts, Fr> {
        self.transport_unwrap.packet_to_frame(packet, timestamp)
    }
}

impl<Ts, Fr, Tu, Fu, F, T> FramingUnwrap<Ts, Fr> for Pipeline<Tu, Fu, F, T>
where
    Fu: FramingUnwrap<Ts, Fr>,
{
    type MessageKind = Fu::MessageKind;
    fn frame_to_message(
        &self,
        frame: TimedData<Ts, Fr>,
    ) -> Option<(crate::TimedPayload<Ts>, Self::MessageKind)> {
        self.frame_unwrap.frame_to_message(frame)
    }
}

impl<Ts, Fr, Tu, Fu, F, T> Framing<Ts, Fr> for Pipeline<Tu, Fu, F, T>
where
    F: Framing<Ts, Fr>,
{
    const OVERHEAD_SIZE: usize = F::OVERHEAD_SIZE;

    fn to_frame(&self, payload: TimedPayload<Ts>, frame_size: usize) -> Vec<TimedData<Ts, Fr>> {
        self.framing.to_frame(payload, frame_size)
    }
}

impl<Ts, Fr, Pkt, Tu, Fu, F, T> Transport<Ts, Fr, Pkt> for Pipeline<Tu, Fu, F, T>
where
    T: Transport<Ts, Fr, Pkt>,
{
    const OVERHEAD_SIZE: usize = T::OVERHEAD_SIZE;

    fn to_transport_packet(&self, frame: TimedData<Ts, Fr>) -> TimedData<Ts, Pkt> {
        self.transport.to_transport_packet(frame)
    }
}

impl<Ts, Fr, Pkt, Tu, Fu, F, T> ProcessingPipeline<Ts, Fr, Pkt> for Pipeline<Tu, Fu, F, T>
where
    Ts: Clone,
    F: Framing<Ts, Fr>,
    T: Transport<Ts, Fr, Pkt>,
{
    fn packet_size(&self) -> usize {
        self.packet_size
    }
}

impl<Ts, Fr, Pkt, Tu, Fu, F, T> UnwrappingPipeline<Ts, Fr, Pkt> for Pipeline<Tu, Fu, F, T>
where
    Ts: Clone,
    Tu: TransportUnwrap<Ts, Fr, Pkt>,
    Fu: FramingUnwrap<Ts, Fr>,
{
}
