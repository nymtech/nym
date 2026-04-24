// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::common::traits::{Framing, Transport};
use crate::mixnodes::traits::ProcessingPipeline;
use crate::{TimedData, TimedPayload};

/// The generic pipeline struct for a mixnode
pub struct Pipeline<F, T> {
    pub packet_size: usize,
    pub framing: F,
    pub transport: T,
}

impl<Ts, Fr, F, T> Framing<Ts, Fr> for Pipeline<F, T>
where
    F: Framing<Ts, Fr>,
{
    const OVERHEAD_SIZE: usize = F::OVERHEAD_SIZE;

    fn to_frame(&self, payload: TimedPayload<Ts>, frame_size: usize) -> Vec<TimedData<Ts, Fr>> {
        self.framing.to_frame(payload, frame_size)
    }
}

impl<Ts, Fr, Pkt, F, T> Transport<Ts, Fr, Pkt> for Pipeline<F, T>
where
    T: Transport<Ts, Fr, Pkt>,
{
    const OVERHEAD_SIZE: usize = T::OVERHEAD_SIZE;

    fn to_transport_packet(&self, frame: TimedData<Ts, Fr>) -> TimedData<Ts, Pkt> {
        self.transport.to_transport_packet(frame)
    }
}

impl<Ts, Fr, Pkt, F, T> ProcessingPipeline<Ts, Fr, Pkt> for Pipeline<F, T>
where
    Ts: Clone,
    F: Framing<Ts, Fr>,
    T: Transport<Ts, Fr, Pkt>,
{
    fn packet_size(&self) -> usize {
        self.packet_size
    }
}
