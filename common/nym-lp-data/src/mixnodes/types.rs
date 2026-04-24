// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::common::traits::{Framing, Transport, WireWrappingPipeline};
use crate::{AddressedTimedData, AddressedTimedPayload};

/// The generic pipeline struct for a mixnode
pub struct Pipeline<F, T, NdId> {
    pub packet_size: usize,
    pub framing: F,
    pub transport: T,
    _marker: std::marker::PhantomData<NdId>,
}

impl<Ts, Fr, F, T, NdId> Framing<Ts, Fr, NdId> for Pipeline<F, T, NdId>
where
    F: Framing<Ts, Fr, NdId>,
{
    const OVERHEAD_SIZE: usize = F::OVERHEAD_SIZE;

    fn to_frame(
        &self,
        payload: AddressedTimedPayload<Ts, NdId>,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<Ts, Fr, NdId>> {
        self.framing.to_frame(payload, frame_size)
    }
}

impl<Ts, Fr, Pkt, F, T, NdId> Transport<Ts, Fr, Pkt, NdId> for Pipeline<F, T, NdId>
where
    T: Transport<Ts, Fr, Pkt, NdId>,
{
    const OVERHEAD_SIZE: usize = T::OVERHEAD_SIZE;

    fn to_transport_packet(
        &self,
        frame: AddressedTimedData<Ts, Fr, NdId>,
    ) -> AddressedTimedData<Ts, Pkt, NdId> {
        self.transport.to_transport_packet(frame)
    }
}

impl<Ts, Fr, Pkt, F, T, NdId> WireWrappingPipeline<Ts, Fr, Pkt, NdId> for Pipeline<F, T, NdId>
where
    Ts: Clone,
    NdId: Clone,
    F: Framing<Ts, Fr, NdId>,
    T: Transport<Ts, Fr, Pkt, NdId>,
{
    fn packet_size(&self) -> usize {
        self.packet_size
    }
}
