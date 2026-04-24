// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp::packet::{
    LpFrame, LpHeader, LpPacket,
    frame::{LpFrameHeader, LpFrameKind},
};
use nym_lp_data::clients::traits::{Chunking, Obfuscation, Reliability, RoutingSecurity};
use nym_lp_data::common::traits::{Framing, Transport};
use nym_lp_data::{TimedData, TimedPayload};

pub type TimedLpFrame<Ts> = TimedData<Ts, LpFrame>;
pub type TimedLpPacket<Ts> = TimedData<Ts, LpPacket>;

pub struct MockChunking;
impl<Ts> Chunking<Ts> for MockChunking
where
    Ts: Clone,
{
    fn chunked(&self, input: Vec<u8>, chunk_size: usize, timestamp: Ts) -> Vec<TimedPayload<Ts>> {
        input
            .chunks(chunk_size)
            .map(|chunk| TimedData {
                data: chunk.to_vec(),
                timestamp: timestamp.clone(),
            })
            .collect()
    }
}

pub struct KcpReliability;

impl KcpReliability {
    const HEADER: &[u8; 5] = b"0KCP0";
}

impl<Ts> Reliability<Ts> for KcpReliability {
    const OVERHEAD_SIZE: usize = Self::HEADER.len();
    fn reliable_encode(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts> {
        input.data_transform(|data| {
            let mut packet = Self::HEADER.to_vec();
            packet.extend(data);
            packet
        })
    }
}

pub struct SphinxSecurity {
    pub nb_frames: usize,
}

impl SphinxSecurity {
    const HEADER: &[u8; 8] = b"0SPHINX0";
}

impl<Ts> RoutingSecurity<Ts> for SphinxSecurity {
    const OVERHEAD_SIZE: usize = Self::HEADER.len();

    fn nb_frames(&self) -> usize {
        self.nb_frames
    }

    fn encrypt(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts> {
        input.data_transform(|data| {
            let mut packet = Self::HEADER.to_vec();
            packet.extend(data);
            packet
        })
    }
}

pub struct KekwObfuscation;

impl Obfuscation<u32> for KekwObfuscation {
    fn obfuscate(
        &mut self,
        input: Option<TimedPayload<u32>>,
        _timestamp: u32,
    ) -> Vec<TimedPayload<u32>> {
        if let Some(input) = input {
            vec![input.ts_transform(|ts| ts + 1)]
        } else {
            Vec::new()
        }
    }
    fn buffer_size(&self) -> usize {
        0
    }
}

pub struct ReallyOddObfuscation {
    next_ts: u32,
}

impl ReallyOddObfuscation {
    pub fn new(start_ts: u32) -> Self {
        let next_ts = if !start_ts.is_multiple_of(2) {
            start_ts
        } else {
            start_ts + 1
        };
        Self { next_ts }
    }
}

impl Obfuscation<u32> for ReallyOddObfuscation {
    fn obfuscate(
        &mut self,
        input: Option<TimedPayload<u32>>,
        _timestamp: u32,
    ) -> Vec<TimedPayload<u32>> {
        if let Some(input) = input {
            let pkt = input.ts_transform(|_| self.next_ts);
            self.next_ts += 2;
            vec![pkt]
        } else {
            Vec::new()
        }
    }
    fn buffer_size(&self) -> usize {
        0
    }
}

pub struct LpFraming;

impl LpFraming {
    const FRAME_ATTRIBUTES: &[u8; 14] = b"0LpFrameAttrs0";
}

impl<Ts> Framing<Ts, LpFrame> for LpFraming
where
    Ts: Clone,
{
    const OVERHEAD_SIZE: usize = LpFrameHeader::SIZE;
    fn to_frame(&self, input: TimedPayload<Ts>, frame_size: usize) -> Vec<TimedLpFrame<Ts>> {
        input
            .data
            .chunks(frame_size)
            .map(|frame_payload| {
                let header = LpFrameHeader::new(LpFrameKind::Opaque, *Self::FRAME_ATTRIBUTES);

                TimedData {
                    data: LpFrame {
                        header,
                        content: frame_payload.to_vec().into(),
                    },
                    timestamp: input.timestamp.clone(),
                }
            })
            .collect()
    }
}

pub struct LpTransport;

impl<Ts> Transport<Ts, LpFrame, LpPacket> for LpTransport {
    const OVERHEAD_SIZE: usize = LpHeader::SIZE;
    fn to_transport_packet(&self, input: TimedLpFrame<Ts>) -> TimedLpPacket<Ts> {
        TimedData {
            data: LpPacket::new(LpHeader::new(7, 7, 7), input.data),
            timestamp: input.timestamp,
        }
    }
}
