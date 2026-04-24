// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::clients::traits::{
    Chunking, ClientWrappingPipeline, Obfuscation, Reliability, RoutingSecurity,
};
use crate::clients::{InputOptions, PipelinePayload};
use crate::common::traits::{Framing, Transport, WireWrappingPipeline};
use crate::{AddressedTimedData, AddressedTimedPayload};

/// Generic composition struct that implements [`ClientWrappingPipeline`] by
/// delegating each stage to a held component.
///
/// Type parameters correspond to the six pipeline stages:
/// - `C`: [`Chunking`]
/// - `R`: [`Reliability`]
/// - `O`: [`Obfuscation`]
/// - `Rs`: [`RoutingSecurity`]
/// - `F`: [`Framing`]
/// - `T`: [`Transport`]
pub struct Pipeline<C, R, O, Rs, F, T> {
    pub packet_size: usize,
    pub chunking: C,
    pub reliability: R,
    pub obfuscation: O,
    pub security: Rs,
    pub framing: F,
    pub transport: T,
}

impl<Ts, Opts, NdId, C, R, O, Rs, F, T> Chunking<Ts, Opts, NdId> for Pipeline<C, R, O, Rs, F, T>
where
    Opts: InputOptions<NdId>,
    C: Chunking<Ts, Opts, NdId>,
{
    fn chunked(
        &mut self,
        input: Vec<u8>,
        input_options: Opts,
        chunk_size: usize,
        timestamp: Ts,
    ) -> Vec<PipelinePayload<Ts, Opts, NdId>> {
        self.chunking
            .chunked(input, input_options, chunk_size, timestamp)
    }
}

impl<Ts, Opts, NdId, C, R, O, Rs, F, T> Reliability<Ts, Opts, NdId> for Pipeline<C, R, O, Rs, F, T>
where
    Opts: InputOptions<NdId>,
    R: Reliability<Ts, Opts, NdId>,
{
    const OVERHEAD_SIZE: usize = R::OVERHEAD_SIZE;

    fn reliable_encode(
        &mut self,
        input: Option<PipelinePayload<Ts, Opts, NdId>>,
        timestamp: Ts,
    ) -> Vec<PipelinePayload<Ts, Opts, NdId>> {
        self.reliability.reliable_encode(input, timestamp)
    }
}

impl<Ts, Opts, NdId, C, R, O, Rs, F, T> Obfuscation<Ts, Opts, NdId> for Pipeline<C, R, O, Rs, F, T>
where
    Opts: InputOptions<NdId>,
    O: Obfuscation<Ts, Opts, NdId>,
{
    fn obfuscate(
        &mut self,
        input: Option<PipelinePayload<Ts, Opts, NdId>>,
        timestamp: Ts,
    ) -> Vec<PipelinePayload<Ts, Opts, NdId>> {
        self.obfuscation.obfuscate(input, timestamp)
    }
}

impl<Ts, Opts, NdId, C, R, O, Rs, F, T> RoutingSecurity<Ts, Opts, NdId>
    for Pipeline<C, R, O, Rs, F, T>
where
    Opts: InputOptions<NdId>,
    Rs: RoutingSecurity<Ts, Opts, NdId>,
{
    const OVERHEAD_SIZE: usize = Rs::OVERHEAD_SIZE;

    fn nb_frames(&self) -> usize {
        self.security.nb_frames()
    }

    fn encrypt(
        &mut self,
        input: PipelinePayload<Ts, Opts, NdId>,
    ) -> PipelinePayload<Ts, Opts, NdId> {
        self.security.encrypt(input)
    }
}

impl<Ts, Fr, NdId, C, R, O, Rs, F, T> Framing<Ts, Fr, NdId> for Pipeline<C, R, O, Rs, F, T>
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

impl<Ts, Fr, Pkt, NdId, C, R, O, Rs, F, T> Transport<Ts, Fr, Pkt, NdId>
    for Pipeline<C, R, O, Rs, F, T>
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

impl<Ts, Fr, Pkt, NdId, C, R, O, Rs, F, T> WireWrappingPipeline<Ts, Fr, Pkt, NdId>
    for Pipeline<C, R, O, Rs, F, T>
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

impl<Ts, Fr, Pkt, Opts, NdId, C, R, O, Rs, F, T> ClientWrappingPipeline<Ts, Fr, Pkt, Opts, NdId>
    for Pipeline<C, R, O, Rs, F, T>
where
    Ts: Clone,
    NdId: Clone,
    Opts: InputOptions<NdId>,
    C: Chunking<Ts, Opts, NdId>,
    R: Reliability<Ts, Opts, NdId>,
    O: Obfuscation<Ts, Opts, NdId>,
    Rs: RoutingSecurity<Ts, Opts, NdId>,
    F: Framing<Ts, Fr, NdId>,
    T: Transport<Ts, Fr, Pkt, NdId>,
{
}
