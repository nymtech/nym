// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Pipeline for nodes operating purely as mixnodes (no client forwarding).
//!

use std::{sync::Arc, time::Instant};

use nym_lp_data::{
    AddressedTimedData, PipelinePayload, TimedData, TimedPayload,
    common::traits::{Framing, FramingUnwrap, Transport, TransportUnwrap},
    nymnodes::traits::NymNodeProcessingPipeline,
    packet::{EncryptedLpPacket, LpFrame, frame::LpFrameHeader},
};
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use rand::Rng;
use tracing::warn;

use crate::node::{
    lp::data::{
        handler::{
            messages::MixMessage,
            pipeline::{
                NymNodeDataPipeline,
                wire::{FramingPipeline, LpTransport},
            },
        },
        shared::SharedLpDataState,
    },
    lp::error::LpHandlerError,
    routing_filter::RoutingFilter,
};

pub(crate) struct MixingNodeDataPipeline<R> {
    state: Arc<SharedLpDataState>,
    framing: FramingPipeline<R>,
}

impl<R: Rng> MixingNodeDataPipeline<R> {
    pub(crate) fn new(state: Arc<SharedLpDataState>, rng: R) -> Self {
        Self {
            state: state.clone(),
            framing: FramingPipeline::new(state, rng),
        }
    }
}

// Processing logic
impl<R: Rng> NymNodeProcessingPipeline<LpFrame> for MixingNodeDataPipeline<R> {
    /// The LP MTU less everything the transport wrap will add on the way out.
    fn frame_size(&self) -> usize {
        nym_lp_data::packet::MTU - EncryptedLpPacket::OVERHEAD
    }

    type Options = MixMessage;
    type MessageKind = MixMessage;

    fn mix(
        &mut self,
        message_kind: MixMessage,
        payload: TimedPayload,
        _: Instant,
    ) -> Vec<PipelinePayload<MixMessage>> {
        // Everything specific to a given packet type should happen here
        let processing_result =
            NymNodeDataPipeline::<R>::process_mix_packet(&self.state, message_kind, payload);

        self.state.update_processing_metrics(&processing_result);

        let packet_to_forward = match processing_result {
            Ok(packet) => packet,
            Err(e) => {
                warn!("Error processing {message_kind:?} packet : {e}");
                return Vec::new();
            }
        };

        // Now we are deciding if we are routing the packet and where

        match packet_to_forward.dst {
            NymNodeRoutingAddress::Node(next_hop) => {
                if !self.state.routing_filter.should_route(next_hop.ip(), false) {
                    // SW need to pipe a socketaddr from the pipeline input
                    warn!(
                        event = "packet.dropped.routing_filter",
                        next_hop = %next_hop,
                        "dropping packet: egress address does not belong to any known node"
                    );
                    self.state.routing_filter_dropped(next_hop);
                    Vec::new()
                } else {
                    vec![packet_to_forward.with_dst(next_hop)]
                }
            }
            NymNodeRoutingAddress::Client(_) => {
                warn!(
                    event = "packet.dropped.client_forwarding_disabled",
                    "dropping packet destined to a client_address on a client_forwarding_disabled node"
                );
                self.state.client_forwarding_disabled_dropped();
                Vec::new()
            }
        }
    }
}

// ============== Framing: delegation to FramingPipeline ==============

impl<R: Rng> Framing<MixMessage> for MixingNodeDataPipeline<R> {
    type Frame = LpFrame;
    const OVERHEAD_SIZE: usize = LpFrameHeader::SIZE;

    fn to_frame(
        &mut self,
        payload: PipelinePayload<MixMessage>,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<Self::Frame>> {
        let frame = LpFrame {
            header: payload.options.into(),
            content: payload.data.data.into(),
        };
        self.framing
            .message_to_frame(payload.data.timestamp, frame, payload.dst, frame_size)
    }
}

impl<R: Rng> Transport<EncryptedLpPacket> for MixingNodeDataPipeline<R> {
    type Frame = LpFrame;
    type Error = LpHandlerError;
    const OVERHEAD_SIZE: usize = EncryptedLpPacket::OVERHEAD;

    fn to_transport_packet(
        &mut self,
        frame: AddressedTimedData<Self::Frame>,
    ) -> Result<AddressedTimedData<EncryptedLpPacket>, Self::Error> {
        LpTransport::frame_to_packet(&self.state, frame)
    }
}

impl<R: Rng> TransportUnwrap<EncryptedLpPacket> for MixingNodeDataPipeline<R> {
    type Frame = LpFrame;
    type Error = LpHandlerError;

    fn packet_to_frame(
        &mut self,
        packet: EncryptedLpPacket,
        timestamp: Instant,
    ) -> Result<TimedData<Self::Frame>, Self::Error> {
        LpTransport::packet_to_frame(&self.state, packet, timestamp)
    }
}

impl<R: Rng> FramingUnwrap<MixMessage> for MixingNodeDataPipeline<R> {
    type Frame = LpFrame;

    fn frame_to_message(
        &mut self,
        frame: TimedData<Self::Frame>,
    ) -> Option<(TimedPayload, MixMessage)> {
        let reassembled = self.framing.frame_to_maybe_message(frame)?;
        let message_kind = reassembled
            .data
            .header
            .try_into()
            .inspect_err(|e| warn!("{e}"))
            .ok()?;

        self.state.message_received(message_kind);
        Some((
            TimedPayload::new(reassembled.timestamp, reassembled.data.content.to_vec()),
            message_kind,
        ))
    }
}
