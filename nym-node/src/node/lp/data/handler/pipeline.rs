// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    net::SocketAddr,
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
use rand::Rng;
use tracing::warn;

use crate::node::lp::data::{
    handler::{messages::MixMessage, processing},
    shared::SharedLpDataState,
};

#[derive(Clone, Copy, Debug)]
pub struct MixnodeDataPipelineConfig {
    pub fragment_timeout: Duration,
}

pub struct MixnodeDataPipeline<R>
where
    R: Rng,
{
    config: MixnodeDataPipelineConfig,
    /// Shared data state
    state: SharedLpDataState,
    fragment_reconstructor: MessageReconstructor<Instant, Duration>,
    rng: R,
}

impl<R> MixnodeDataPipeline<R>
where
    R: Rng,
{
    pub fn new(state: SharedLpDataState, config: MixnodeDataPipelineConfig, rng: R) -> Self {
        Self {
            state,
            config,
            rng,
            fragment_reconstructor: MessageReconstructor::new(config.fragment_timeout),
        }
    }
}

// Mixing logic
impl<R> MixnodeProcessingPipeline<Instant, EncryptedLpPacket, MixMessage, MixMessage, SocketAddr>
    for MixnodeDataPipeline<R>
where
    R: Rng,
{
    fn mix(
        &mut self,
        message_kind: MixMessage,
        payload: TimedPayload<Instant>,
        _: Instant,
    ) -> Vec<PipelinePayload<Instant, MixMessage, SocketAddr>> {
        let processing_result = match message_kind {
            MixMessage::Sphinx {
                key_rotation,
                reserved: _,
            } => match processing::sphinx::process(&self.state, payload, key_rotation) {
                Ok(packet) => packet,
                Err(e) => {
                    warn!("Error processing sphinx packet : {e}");
                    return Vec::new();
                }
            },
            MixMessage::Outfox {
                key_rotation,
                reserved: _,
            } => match processing::outfox::process(&self.state, payload, key_rotation) {
                Ok(packet) => packet,
                Err(e) => {
                    warn!("Error processing outfox packet : {e}");
                    return Vec::new();
                }
            },
        };

        vec![processing_result.with_options(message_kind)]
    }
}

impl<R> Framing<Instant, MixMessage, SocketAddr> for MixnodeDataPipeline<R>
where
    R: Rng,
{
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

impl<R> Transport<Instant, EncryptedLpPacket, SocketAddr> for MixnodeDataPipeline<R>
where
    R: Rng,
{
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

impl<R> WireWrappingPipeline<Instant, EncryptedLpPacket, MixMessage, SocketAddr>
    for MixnodeDataPipeline<R>
where
    R: Rng,
{
    fn packet_size(&self) -> usize {
        nym_lp_data::packet::MTU
    }
}

impl<R> TransportUnwrap<Instant, EncryptedLpPacket> for MixnodeDataPipeline<R>
where
    R: Rng,
{
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

impl<R> FramingUnwrap<Instant, MixMessage> for MixnodeDataPipeline<R>
where
    R: Rng,
{
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
            let (payload, metadata) = self
                .fragment_reconstructor
                .insert_new_fragment(fragment, frame.timestamp)?;
            let message_kind = metadata
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

impl<R> WireUnwrappingPipeline<Instant, EncryptedLpPacket, MixMessage> for MixnodeDataPipeline<R> where
    R: Rng
{
}
