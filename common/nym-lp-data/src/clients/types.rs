// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;

use crate::clients::traits::{
    Chunking, Obfuscation, ProcessingPipeline, Reliability, RoutingSecurity,
};
use crate::common::traits::{Framing, Transport};
use crate::{TimedData, TimedPayload};

#[derive(Clone, Copy, Debug)]
pub struct StreamOptions {
    pub reliability: bool,
    pub security: bool,
    pub obfuscation: bool,
}

impl Default for StreamOptions {
    fn default() -> Self {
        Self {
            reliability: true,
            security: true,
            obfuscation: true,
        }
    }
}

/// The generic pipeline struct
pub struct Pipeline<C, R, O, Rs, F, T> {
    pub packet_size: usize,
    pub chunking: C,
    pub reliability: R,
    pub obfuscation: O,
    pub security: Rs,
    pub framing: F,
    pub transport: T,
}

impl<Ts, C, R, O, Rs, F, T> Chunking<Ts> for Pipeline<C, R, O, Rs, F, T>
where
    C: Chunking<Ts>,
{
    fn chunked(&self, input: Vec<u8>, chunk_size: usize, timestamp: Ts) -> Vec<TimedPayload<Ts>> {
        self.chunking.chunked(input, chunk_size, timestamp)
    }
}

impl<Ts, C, R, O, Rs, F, T> Reliability<Ts> for Pipeline<C, R, O, Rs, F, T>
where
    R: Reliability<Ts>,
{
    const OVERHEAD_SIZE: usize = R::OVERHEAD_SIZE;

    fn reliable_encode(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts> {
        self.reliability.reliable_encode(input)
    }
}

impl<Ts, C, R, O, Rs, F, T> Obfuscation<Ts> for Pipeline<C, R, O, Rs, F, T>
where
    O: Obfuscation<Ts>,
{
    fn obfuscate(&mut self, input: TimedPayload<Ts>, timestamp: Ts) -> Vec<TimedPayload<Ts>> {
        self.obfuscation.obfuscate(input, timestamp)
    }
    fn buffer_size(&self) -> usize {
        self.obfuscation.buffer_size()
    }
}

impl<Ts, C, R, O, Rs, F, T> RoutingSecurity<Ts> for Pipeline<C, R, O, Rs, F, T>
where
    Rs: RoutingSecurity<Ts>,
{
    const OVERHEAD_SIZE: usize = Rs::OVERHEAD_SIZE;
    fn nb_frames(&self) -> usize {
        self.security.nb_frames()
    }

    fn encrypt(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts> {
        self.security.encrypt(input)
    }
}

impl<Ts, Fr, C, R, O, Rs, F, T> Framing<Ts, Fr> for Pipeline<C, R, O, Rs, F, T>
where
    F: Framing<Ts, Fr>,
{
    const OVERHEAD_SIZE: usize = F::OVERHEAD_SIZE;

    fn to_frame(&self, payload: TimedPayload<Ts>, frame_size: usize) -> Vec<TimedData<Ts, Fr>> {
        self.framing.to_frame(payload, frame_size)
    }
}

impl<Ts, Fr, Pkt, C, R, O, Rs, F, T> Transport<Ts, Fr, Pkt> for Pipeline<C, R, O, Rs, F, T>
where
    T: Transport<Ts, Fr, Pkt>,
{
    const OVERHEAD_SIZE: usize = T::OVERHEAD_SIZE;

    fn to_transport_packet(&self, frame: TimedData<Ts, Fr>) -> TimedData<Ts, Pkt> {
        self.transport.to_transport_packet(frame)
    }
}

impl<Ts, Fr, Pkt, C, R, O, Rs, F, T> ProcessingPipeline<Ts, Fr, Pkt> for Pipeline<C, R, O, Rs, F, T>
where
    Ts: Clone,
    C: Chunking<Ts>,
    R: Reliability<Ts>,
    O: Obfuscation<Ts>,
    Rs: RoutingSecurity<Ts>,
    F: Framing<Ts, Fr>,
    T: Transport<Ts, Fr, Pkt>,
{
    fn packet_size(&self) -> usize {
        self.packet_size
    }
}
