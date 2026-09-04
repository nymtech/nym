// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Instant;

use crate::clients::traits::{
    Chunking, ClientWrappingPipeline, Obfuscation, Reliability, RoutingSecurity,
};
use crate::common::traits::{Framing, Transport, WireWrappingPipeline};
use crate::{AddressedTimedData, PipelinePayload};

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
    /// On-wire size of an output packet in bytes; returned by
    /// [`WireWrappingPipeline::packet_size`].
    pub packet_size: usize,
    /// [`Chunking`] stage.
    pub chunking: C,
    /// [`Reliability`] stage.
    pub reliability: R,
    /// [`Obfuscation`] stage.
    pub obfuscation: O,
    /// [`RoutingSecurity`] stage.
    pub security: Rs,
    /// [`Framing`] stage.
    pub framing: F,
    /// [`Transport`] stage.
    pub transport: T,
}

impl<Opts, C, R, O, Rs, F, T> Chunking<Opts> for Pipeline<C, R, O, Rs, F, T>
where
    C: Chunking<Opts>,
{
    fn chunked(
        &mut self,
        input: PipelinePayload<Opts>,
        chunk_size: usize,
        timestamp: Instant,
    ) -> Vec<PipelinePayload<Opts>> {
        self.chunking.chunked(input, chunk_size, timestamp)
    }
}

impl<Opts, C, R, O, Rs, F, T> Reliability<Opts> for Pipeline<C, R, O, Rs, F, T>
where
    R: Reliability<Opts>,
{
    const OVERHEAD_SIZE: usize = R::OVERHEAD_SIZE;

    fn reliable_encode(
        &mut self,
        input: Option<PipelinePayload<Opts>>,
        timestamp: Instant,
    ) -> Vec<PipelinePayload<Opts>> {
        self.reliability.reliable_encode(input, timestamp)
    }
}

impl<Opts, C, R, O, Rs, F, T> Obfuscation<Opts> for Pipeline<C, R, O, Rs, F, T>
where
    O: Obfuscation<Opts>,
{
    fn obfuscate(
        &mut self,
        input: Option<PipelinePayload<Opts>>,
        timestamp: Instant,
    ) -> Vec<PipelinePayload<Opts>> {
        self.obfuscation.obfuscate(input, timestamp)
    }
}

impl<Opts, C, R, O, Rs, F, T> RoutingSecurity<Opts> for Pipeline<C, R, O, Rs, F, T>
where
    Rs: RoutingSecurity<Opts>,
{
    const OVERHEAD_SIZE: usize = Rs::OVERHEAD_SIZE;

    fn nb_frames(&self) -> usize {
        self.security.nb_frames()
    }

    fn encrypt(&mut self, input: PipelinePayload<Opts>) -> PipelinePayload<Opts> {
        self.security.encrypt(input)
    }
}

impl<Opts, C, R, O, Rs, F, T> Framing<Opts> for Pipeline<C, R, O, Rs, F, T>
where
    F: Framing<Opts>,
{
    type Frame = F::Frame;
    const OVERHEAD_SIZE: usize = F::OVERHEAD_SIZE;

    fn to_frame(
        &mut self,
        payload: PipelinePayload<Opts>,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<F::Frame>> {
        self.framing.to_frame(payload, frame_size)
    }
}

impl<Pkt, C, R, O, Rs, F, T> Transport<Pkt> for Pipeline<C, R, O, Rs, F, T>
where
    T: Transport<Pkt>,
{
    type Frame = T::Frame;
    type Error = T::Error;
    const OVERHEAD_SIZE: usize = T::OVERHEAD_SIZE;

    fn to_transport_packet(
        &mut self,
        frame: AddressedTimedData<T::Frame>,
    ) -> Result<AddressedTimedData<Pkt>, Self::Error> {
        self.transport.to_transport_packet(frame)
    }
}

impl<Pkt, Opts, C, R, O, Rs, F, T> WireWrappingPipeline<Pkt, Opts> for Pipeline<C, R, O, Rs, F, T>
where
    F: Framing<Opts>,
    T: Transport<Pkt, Frame = F::Frame>,
{
    fn packet_size(&self) -> usize {
        self.packet_size
    }
}

impl<Pkt, Opts, C, R, O, Rs, F, T> ClientWrappingPipeline<Pkt, Opts> for Pipeline<C, R, O, Rs, F, T>
where
    C: Chunking<Opts>,
    R: Reliability<Opts>,
    O: Obfuscation<Opts>,
    Rs: RoutingSecurity<Opts>,
    F: Framing<Opts>,
    T: Transport<Pkt, Frame = F::Frame>,
{
}
