// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::{Duration, Instant};

use nym_lp_data::packet::{
    LpFrame, LpHeader, LpPacket,
    frame::{LpFrameHeader, LpFrameKind},
};

use nym_lp_data::{
    AddressedTimedData, PipelinePayload,
    clients::traits::{Chunking, Obfuscation, Reliability, RoutingSecurity},
    common::traits::{Framing, Transport},
};

pub type BasicPipelinePayload = PipelinePayload<()>;

pub struct MockChunking;
impl Chunking<()> for MockChunking {
    fn chunked(
        &mut self,
        input: BasicPipelinePayload,
        chunk_size: usize,
        timestamp: Instant,
    ) -> Vec<BasicPipelinePayload> {
        input
            .data
            .data
            .chunks(chunk_size)
            .map(|chunk| BasicPipelinePayload::new(timestamp, chunk.to_vec(), (), input.dst))
            .collect()
    }
}

pub struct MockReliability;

impl MockReliability {
    const HEADER: &[u8; 5] = b"0KCP0";
}

impl Reliability<()> for MockReliability {
    const OVERHEAD_SIZE: usize = Self::HEADER.len();
    fn reliable_encode(
        &mut self,
        input: Option<BasicPipelinePayload>,
        _: Instant,
    ) -> Vec<BasicPipelinePayload> {
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

pub struct MockSphinxSecurity {
    pub nb_frames: usize,
}

impl MockSphinxSecurity {
    const HEADER: &[u8; 8] = b"0SPHINX0";
}

impl RoutingSecurity<()> for MockSphinxSecurity {
    const OVERHEAD_SIZE: usize = Self::HEADER.len();

    fn nb_frames(&self) -> usize {
        self.nb_frames
    }

    fn encrypt(&mut self, input: BasicPipelinePayload) -> BasicPipelinePayload {
        input.data_transform(|data| {
            let mut packet = Self::HEADER.to_vec();
            packet.extend(data);
            packet
        })
    }
}

pub struct KekwObfuscation;

impl Obfuscation<()> for KekwObfuscation {
    fn obfuscate(
        &mut self,
        input: Option<BasicPipelinePayload>,
        _timestamp: Instant,
    ) -> Vec<BasicPipelinePayload> {
        if let Some(input) = input {
            let new_timestamp = input.data.timestamp + Duration::from_millis(1);
            vec![input.with_timestamp(new_timestamp)]
        } else {
            Vec::new()
        }
    }
}

pub struct MockLpFraming;

impl MockLpFraming {
    const FRAME_ATTRIBUTES: &[u8; 14] = b"0LpFrameAttrs0";
}

impl Framing<()> for MockLpFraming {
    type Frame = LpFrame;
    const OVERHEAD_SIZE: usize = LpFrameHeader::SIZE;
    fn to_frame(
        &mut self,
        input: BasicPipelinePayload,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<LpFrame>> {
        input
            .data
            .data
            .chunks(frame_size)
            .map(|frame_payload| {
                let header = LpFrameHeader::new(LpFrameKind::Opaque, *Self::FRAME_ATTRIBUTES);

                AddressedTimedData::new_addressed(
                    input.data.timestamp,
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

pub struct MockLpTransport;

impl Transport<LpPacket> for MockLpTransport {
    type Frame = LpFrame;
    type Error = std::convert::Infallible;
    const OVERHEAD_SIZE: usize = LpHeader::SIZE;
    fn to_transport_packet(
        &mut self,
        input: AddressedTimedData<Self::Frame>,
    ) -> Result<AddressedTimedData<LpPacket>, Self::Error> {
        Ok(AddressedTimedData::new_addressed(
            input.data.timestamp,
            LpPacket::new(LpHeader::new(7, 7, 7), input.data.data),
            input.dst,
        ))
    }
}
