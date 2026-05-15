// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{Duration, Instant},
};

use nym_lp_data::{
    AddressedTimedData, PipelinePayload, TimedData, TimedPayload,
    common::traits::{
        Framing, FramingUnwrap, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
    fragmentation::{fragment::fragment_payload, reconstruction::MessageReconstructor},
    mixnodes::traits::MixnodeProcessingPipeline,
    packet::{
        EncryptedLpPacket, LpFrame, LpHeader, LpPacket, MalformedLpPacketError,
        frame::LpFrameHeader,
    },
};
use rand::rngs::OsRng;
use tracing::warn;

use crate::node::lp::data::handler::messages::MixMessage;

#[derive(Clone, Copy, Debug)]
pub struct MixnodeDataPipelineConfig {
    pub fragment_timeout: Duration,
}

pub struct MixnodeDataPipeline {
    config: MixnodeDataPipelineConfig,
    fragment_reconstructor: MessageReconstructor<Instant, Duration>,
    rng: OsRng,
}

impl MixnodeDataPipeline {
    pub fn new(config: MixnodeDataPipelineConfig) -> Self {
        Self {
            config,
            rng: OsRng,
            fragment_reconstructor: MessageReconstructor::new(config.fragment_timeout),
        }
    }
}

// Mixing logic
impl MixnodeProcessingPipeline<Instant, EncryptedLpPacket, MixMessage, MixMessage, SocketAddr>
    for MixnodeDataPipeline
{
    fn mix(
        &mut self,
        message_kind: MixMessage,
        payload: TimedPayload<Instant>,
        timestamp: Instant,
    ) -> Vec<PipelinePayload<Instant, MixMessage, SocketAddr>> {
        println!("received a payload : {payload:?}");
        vec![PipelinePayload::new(
            timestamp,
            payload.data,
            message_kind,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9000),
        )]
    }
}

impl Framing<Instant, MixMessage, SocketAddr> for MixnodeDataPipeline {
    type Frame = LpFrame;

    const OVERHEAD_SIZE: usize = LpFrameHeader::SIZE;

    fn to_frame(
        &mut self,
        payload: PipelinePayload<Instant, MixMessage, SocketAddr>,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<Instant, Self::Frame, SocketAddr>> {
        let content = payload.data.data;
        let fragments =
            fragment_payload(&mut self.rng, &content, payload.options.into(), frame_size);

        fragments
            .into_iter()
            .map(|f| {
                AddressedTimedData::new_addressed(
                    payload.data.timestamp,
                    f.into_lp_frame(),
                    payload.dst,
                )
            })
            .collect()
    }
}

impl Transport<Instant, EncryptedLpPacket, SocketAddr> for MixnodeDataPipeline {
    type Frame = LpFrame;

    const OVERHEAD_SIZE: usize = LpHeader::SIZE;

    fn to_transport_packet(
        &mut self,
        frame: AddressedTimedData<Instant, Self::Frame, SocketAddr>,
    ) -> AddressedTimedData<Instant, EncryptedLpPacket, SocketAddr> {
        // Here be LP encryption. For not, just wrap into an EncryptedLpPacket, we don't care at reception anyway
        frame.data_transform(|f| LpPacket::new(LpHeader::new(0, 0, 0), f).encode())
    }
}

impl WireWrappingPipeline<Instant, EncryptedLpPacket, MixMessage, SocketAddr>
    for MixnodeDataPipeline
{
    fn packet_size(&self) -> usize {
        nym_lp_data::packet::MTU
    }
}

impl TransportUnwrap<Instant, EncryptedLpPacket> for MixnodeDataPipeline {
    type Frame = LpFrame;
    type Error = MalformedLpPacketError;

    fn packet_to_frame(
        &mut self,
        packet: EncryptedLpPacket,
        timestamp: Instant,
    ) -> Result<TimedData<Instant, Self::Frame>, Self::Error> {
        // Here be LP decryption. For now we do as is it's not encrypted
        let lp_packet = LpPacket::decode(packet)?;
        Ok(TimedData {
            timestamp,
            data: lp_packet.into_frame(),
        })
    }
}

impl FramingUnwrap<Instant, MixMessage> for MixnodeDataPipeline {
    type Frame = LpFrame;
    fn frame_to_message(
        &mut self,
        frame: TimedData<Instant, Self::Frame>,
    ) -> Option<(TimedPayload<Instant>, MixMessage)> {
        if frame.data.kind().is_fragmented() {
            let fragment = frame
                .data
                .try_into()
                .inspect_err(|e| tracing::error!("Failed to recover a fragment : {e}"))
                .ok()?;
            let (payload, frame_kind) = self
                .fragment_reconstructor
                .insert_new_fragment(fragment, frame.timestamp)?;
            let message_kind = frame_kind
                .try_into()
                .inspect_err(|e| tracing::warn!("{e}"))
                .ok()?;
            Some((TimedPayload::new(frame.timestamp, payload), message_kind))
        } else {
            warn!("unimplemented yet");
            None
        }
    }
}

impl WireUnwrappingPipeline<Instant, EncryptedLpPacket, MixMessage> for MixnodeDataPipeline {}
