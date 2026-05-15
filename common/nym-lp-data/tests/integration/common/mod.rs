// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp::packet::{
    LpFrame, LpHeader, LpPacket,
    frame::{LpFrameHeader, LpFrameKind},
};

use nym_lp_data::{
    AddressedTimedData, PipelinePayload,
    clients::{
        InputOptions,
        traits::{Chunking, Obfuscation, Reliability, RoutingSecurity},
    },
    common::traits::{Framing, Transport},
};

#[derive(Clone, Copy)]
pub struct BasicOptions {
    pub reliability: bool,
    pub security: bool,
    pub obfuscation: bool,
    pub next_hop: u8,
}

impl InputOptions<u8> for BasicOptions {
    fn reliability(&self) -> bool {
        self.reliability
    }

    fn routing_security(&self) -> bool {
        self.security
    }

    fn obfuscation(&self) -> bool {
        self.obfuscation
    }

    fn next_hop(&self) -> u8 {
        self.next_hop
    }
}

pub type BasicPipelinePayload<Ts> = PipelinePayload<Ts, BasicOptions, u8>;

pub struct MockChunking;
impl<Ts> Chunking<Ts, BasicOptions, u8> for MockChunking
where
    Ts: Clone,
{
    fn chunked(
        &mut self,
        input: Vec<u8>,
        input_options: BasicOptions,
        chunk_size: usize,
        timestamp: Ts,
    ) -> Vec<BasicPipelinePayload<Ts>> {
        input
            .chunks(chunk_size)
            .map(|chunk| {
                BasicPipelinePayload::new(
                    timestamp.clone(),
                    chunk.to_vec(),
                    input_options,
                    input_options.next_hop(),
                )
            })
            .collect()
    }
}

pub struct KcpReliability;

impl KcpReliability {
    const HEADER: &[u8; 5] = b"0KCP0";
}

impl<Ts> Reliability<Ts, BasicOptions, u8> for KcpReliability {
    const OVERHEAD_SIZE: usize = Self::HEADER.len();
    fn reliable_encode(
        &mut self,
        input: Option<BasicPipelinePayload<Ts>>,
        _: Ts,
    ) -> Vec<BasicPipelinePayload<Ts>> {
        input
            .map(|data| {
                vec![data.data_transform(|data| {
                    let mut packet = Self::HEADER.to_vec();
                    packet.extend(data);
                    packet
                })]
            })
            .unwrap_or_default()
    }
}

pub struct SphinxSecurity {
    pub nb_frames: usize,
}

impl SphinxSecurity {
    const HEADER: &[u8; 8] = b"0SPHINX0";
}

impl<Ts> RoutingSecurity<Ts, BasicOptions, u8> for SphinxSecurity {
    const OVERHEAD_SIZE: usize = Self::HEADER.len();

    fn nb_frames(&self) -> usize {
        self.nb_frames
    }

    fn encrypt(&mut self, input: BasicPipelinePayload<Ts>) -> BasicPipelinePayload<Ts> {
        input.data_transform(|data| {
            let mut packet = Self::HEADER.to_vec();
            packet.extend(data);
            packet
        })
    }
}

pub struct KekwObfuscation;

impl Obfuscation<u32, BasicOptions, u8> for KekwObfuscation {
    fn obfuscate(
        &mut self,
        input: Option<BasicPipelinePayload<u32>>,
        _timestamp: u32,
    ) -> Vec<BasicPipelinePayload<u32>> {
        if let Some(input) = input {
            vec![input.ts_transform(|ts| ts + 1)]
        } else {
            Vec::new()
        }
    }
}

#[allow(dead_code)]
pub struct ReallyOddObfuscation {
    next_ts: u32,
}

impl ReallyOddObfuscation {
    #[allow(dead_code)]
    pub fn new(start_ts: u32) -> Self {
        let next_ts = if !start_ts.is_multiple_of(2) {
            start_ts
        } else {
            start_ts + 1
        };
        Self { next_ts }
    }
}

impl Obfuscation<u32, BasicOptions, u8> for ReallyOddObfuscation {
    fn obfuscate(
        &mut self,
        input: Option<BasicPipelinePayload<u32>>,
        _timestamp: u32,
    ) -> Vec<BasicPipelinePayload<u32>> {
        if let Some(input) = input {
            let pkt = input.ts_transform(|_| self.next_ts);
            self.next_ts += 2;
            vec![pkt]
        } else {
            Vec::new()
        }
    }
}

pub struct LpFraming;

impl LpFraming {
    const FRAME_ATTRIBUTES: &[u8; 14] = b"0LpFrameAttrs0";
}

impl<Ts> Framing<Ts, BasicOptions, u8> for LpFraming
where
    Ts: Clone,
{
    type Frame = LpFrame;
    const OVERHEAD_SIZE: usize = LpFrameHeader::SIZE;
    fn to_frame(
        &mut self,
        input: PipelinePayload<Ts, BasicOptions, u8>,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<Ts, LpFrame, u8>> {
        input
            .data
            .data
            .chunks(frame_size)
            .map(|frame_payload| {
                let header = LpFrameHeader::new(LpFrameKind::Opaque, *Self::FRAME_ATTRIBUTES);

                AddressedTimedData::new_addressed(
                    input.data.timestamp.clone(),
                    LpFrame {
                        header,
                        content: frame_payload.to_vec().into(),
                    },
                    input.dst,
                )
            })
            .collect()
    }
}

pub struct LpTransport;

impl<Ts> Transport<Ts, LpPacket, u8> for LpTransport {
    type Frame = LpFrame;
    const OVERHEAD_SIZE: usize = LpHeader::SIZE;
    fn to_transport_packet(
        &self,
        input: AddressedTimedData<Ts, Self::Frame, u8>,
    ) -> AddressedTimedData<Ts, LpPacket, u8> {
        AddressedTimedData::new_addressed(
            input.data.timestamp,
            LpPacket::new(LpHeader::new(7, 7, 7), input.data.data),
            input.dst,
        )
    }
}
