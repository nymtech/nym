// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;

use crate::traits::{
    Chunking, Framing, Obfuscation, ProcessingPipeline, Reliability, Security, Transport,
};

pub struct TimedData<P, Ts> {
    pub data: P,
    pub timestamp: Ts,
}

impl<P, Ts> Debug for TimedData<P, Ts>
where
    P: Debug,
    Ts: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "TimedData {{")?;
        writeln!(f, "    data:")?;
        let data_debug = format!("{:#?}", &self.data);
        for line in data_debug.lines() {
            writeln!(f, "        {}", line)?;
        }
        writeln!(f, "    timestamp: {:#?},", &self.timestamp)?;
        write!(f, "}}")
    }
}

impl<P, Ts> TimedData<P, Ts> {
    pub fn data_transform<F>(self, mut op: F) -> Self
    where
        F: FnMut(P) -> P,
    {
        TimedData {
            data: op(self.data),
            timestamp: self.timestamp,
        }
    }

    pub fn ts_transform<F>(self, mut op: F) -> Self
    where
        F: FnMut(Ts) -> Ts,
    {
        TimedData {
            data: self.data,
            timestamp: op(self.timestamp),
        }
    }
}

/// Helper type to erase the Vec<u8> parameters
pub type TimedPayload<Ts> = TimedData<Vec<u8>, Ts>;

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
pub struct Pipeline<C, R, S, O, F, T> {
    pub packet_size: usize,
    pub chunking: C,
    pub reliability: R,
    pub security: S,
    pub obfuscation: O,
    pub framing: F,
    pub transport: T,
}

impl<Ts, C, R, S, O, F, T> Chunking<Ts> for Pipeline<C, R, S, O, F, T>
where
    C: Chunking<Ts>,
{
    fn chunked(&self, input: Vec<u8>, chunk_size: usize, timestamp: Ts) -> Vec<TimedPayload<Ts>> {
        self.chunking.chunked(input, chunk_size, timestamp)
    }
}

impl<Ts, C, R, S, O, F, T> Reliability<Ts> for Pipeline<C, R, S, O, F, T>
where
    R: Reliability<Ts>,
{
    const OVERHEAD_SIZE: usize = R::OVERHEAD_SIZE;

    fn reliable_encode(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts> {
        self.reliability.reliable_encode(input)
    }
}

impl<Ts, C, R, S, O, F, T> Security<Ts> for Pipeline<C, R, S, O, F, T>
where
    S: Security<Ts>,
{
    const OVERHEAD_SIZE: usize = S::OVERHEAD_SIZE;
    fn nb_frames(&self) -> usize {
        self.security.nb_frames()
    }

    fn encrypt(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts> {
        self.security.encrypt(input)
    }
}

impl<Ts, C, R, S, O, F, T> Obfuscation<Ts> for Pipeline<C, R, S, O, F, T>
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

impl<Ts, C, R, S, O, F, T, Fr> Framing<Ts, Fr> for Pipeline<C, R, S, O, F, T>
where
    F: Framing<Ts, Fr>,
{
    const OVERHEAD_SIZE: usize = F::OVERHEAD_SIZE;

    fn to_frame(&self, payload: TimedPayload<Ts>, frame_size: usize) -> Vec<TimedData<Fr, Ts>> {
        self.framing.to_frame(payload, frame_size)
    }
}

impl<Ts, C, R, S, O, F, T, Fr, P> Transport<Ts, Fr, P> for Pipeline<C, R, S, O, F, T>
where
    T: Transport<Ts, Fr, P>,
{
    const OVERHEAD_SIZE: usize = T::OVERHEAD_SIZE;

    fn to_transport_packet(&self, frame: TimedData<Fr, Ts>) -> TimedData<P, Ts> {
        self.transport.to_transport_packet(frame)
    }
}

impl<Ts, C, R, S, O, F, T, Fr, P> ProcessingPipeline<Ts, Fr, P> for Pipeline<C, R, S, O, F, T>
where
    Ts: Clone,
    C: Chunking<Ts>,
    R: Reliability<Ts>,
    S: Security<Ts>,
    O: Obfuscation<Ts>,
    F: Framing<Ts, Fr>,
    T: Transport<Ts, Fr, P>,
{
    fn packet_size(&self) -> usize {
        self.packet_size
    }
}
