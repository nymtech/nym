// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;

use crate::traits::{
    Chunking, Framing, Obfuscation, ProcessingPipeline, Reliability, Security, Transport,
};

pub struct TimedData<Ts, D> {
    pub timestamp: Ts,
    pub data: D,
}

impl<Ts, D> Debug for TimedData<Ts, D>
where
    D: Debug,
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

impl<Ts, D> TimedData<Ts, D> {
    pub fn data_transform<F>(self, mut op: F) -> Self
    where
        F: FnMut(D) -> D,
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
pub type TimedPayload<Ts> = TimedData<Ts, Vec<u8>>;

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
pub struct Pipeline<C, R, O, S, F, T> {
    pub packet_size: usize,
    pub chunking: C,
    pub reliability: R,
    pub obfuscation: O,
    pub security: S,
    pub framing: F,
    pub transport: T,
}

impl<Ts, C, R, O, S, F, T> Chunking<Ts> for Pipeline<C, R, O, S, F, T>
where
    C: Chunking<Ts>,
{
    fn chunked(&self, input: Vec<u8>, chunk_size: usize, timestamp: Ts) -> Vec<TimedPayload<Ts>> {
        self.chunking.chunked(input, chunk_size, timestamp)
    }
}

impl<Ts, C, R, O, S, F, T> Reliability<Ts> for Pipeline<C, R, O, S, F, T>
where
    R: Reliability<Ts>,
{
    const OVERHEAD_SIZE: usize = R::OVERHEAD_SIZE;

    fn reliable_encode(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts> {
        self.reliability.reliable_encode(input)
    }
}

impl<Ts, C, R, O, S, F, T> Obfuscation<Ts> for Pipeline<C, R, O, S, F, T>
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

impl<Ts, C, R, O, S, F, T> Security<Ts> for Pipeline<C, R, O, S, F, T>
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

impl<Ts, Fr, C, R, O, S, F, T> Framing<Ts, Fr> for Pipeline<C, R, O, S, F, T>
where
    F: Framing<Ts, Fr>,
{
    const OVERHEAD_SIZE: usize = F::OVERHEAD_SIZE;

    fn to_frame(&self, payload: TimedPayload<Ts>, frame_size: usize) -> Vec<TimedData<Ts, Fr>> {
        self.framing.to_frame(payload, frame_size)
    }
}

impl<Ts, Fr, Pkt, C, R, O, S, F, T> Transport<Ts, Fr, Pkt> for Pipeline<C, R, O, S, F, T>
where
    T: Transport<Ts, Fr, Pkt>,
{
    const OVERHEAD_SIZE: usize = T::OVERHEAD_SIZE;

    fn to_transport_packet(&self, frame: TimedData<Ts, Fr>) -> TimedData<Ts, Pkt> {
        self.transport.to_transport_packet(frame)
    }
}

impl<Ts, Fr, Pkt, C, R, O, S, F, T> ProcessingPipeline<Ts, Fr, Pkt> for Pipeline<C, R, O, S, F, T>
where
    Ts: Clone,
    C: Chunking<Ts>,
    R: Reliability<Ts>,
    O: Obfuscation<Ts>,
    S: Security<Ts>,
    F: Framing<Ts, Fr>,
    T: Transport<Ts, Fr, Pkt>,
{
    fn packet_size(&self) -> usize {
        self.packet_size
    }
}
