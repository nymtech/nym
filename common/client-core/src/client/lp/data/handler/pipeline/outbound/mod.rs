// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Turning what a client wants to send into packets on the wire.
//!
//! The mirror of [`inbound`](super::inbound), and built from the same six stages every client
//! wrapping pipeline has: chunking, reliability, obfuscation, routing security, framing, transport.
//! Most of them are pass-through here - there are no acknowledgements and no cover traffic on this
//! path yet - but they are the stages, so adding either is filling one in rather than finding a
//! place to put it.
//!
//! # The LP path is not the classic path
//!
//! The entry gateway *forwards* without processing and the recipient client is the last sphinx hop,
//! so packets carry no SURB-ack and no payload wrapper. That is what
//! [`prepare_chunk_for_lp`](nym_sphinx::preparer::MessagePreparer::prepare_chunk_for_lp) does
//! differently from its classic sibling, and why the reliability stage is a no-op rather than an
//! ack-generating one.

use crate::client::lp::data::handler::error::LpDataHandlerError;
use crate::client::lp::data::shared::SharedLpDataState;
use crate::client::topology_control::TopologyAccessor;
use nym_client_core_config_types::DebugConfig;
use std::sync::Arc;

use nym_lp_data::clients::helpers::{NoOpObfuscation, NoOpReliability};
use nym_lp_data::clients::traits::{Chunking, ClientWrappingPipeline};
use nym_lp_data::common::traits::{Framing, Transport, WireWrappingPipeline};
use nym_lp_data::fragmentation::fragment::fragment_lp_message;
use nym_lp_data::packet::frame::LpFrameHeader;
use nym_lp_data::packet::{EncryptedLpPacket, LpFrame, MTU};
use nym_lp_data::{AddressedTimedData, PipelinePayload, TimedData};
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::message::NymMessage;
use rand::rngs::OsRng;
use rand::{CryptoRng, Rng};
use std::time::Instant;
use tracing::warn;

mod routing_security;

/// What the stages need to know about a message that the bytes cannot say.
///
/// The gateway is not in here: it is the payload's destination, which every stage already carries.
#[derive(Clone, Copy, Debug)]
pub struct LpOutboundOptions {
    /// Who the message is for. Every fragment is routed to them separately.
    pub recipient: Recipient,
}

/// Wraps outbound messages for the LP path.
///
/// Runs on a blocking worker. Its stages are synchronous, which the topology accessor is fine with:
/// reads there are wait-free, so they need neither the runtime nor a lock.
pub struct LpOutboundPipeline<R> {
    /// Draws routes, sphinx delays, and the split points of both fragmentations.
    rng: R,

    /// Fixes the route this client picks for a given fragment, when the config asks for it.
    nonce: i32,

    /// What the sphinx layer is built with; read through [`FragmentPreparer`](super::preparer).
    debug_config: DebugConfig,

    /// Where routes come from. Read per fragment, so each takes its own path.
    topology_accessor: TopologyAccessor,

    /// The sessions frames are encrypted on, shared with the inbound direction that decrypts on
    /// them.
    shared_state: Arc<SharedLpDataState>,
}

impl<R> LpOutboundPipeline<R>
where
    R: CryptoRng + Rng,
{
    pub fn new(
        mut rng: R,
        debug_config: DebugConfig,
        topology_accessor: TopologyAccessor,
        shared_state: Arc<SharedLpDataState>,
    ) -> Self {
        LpOutboundPipeline {
            nonce: rng.r#gen(),
            rng,
            debug_config,
            topology_accessor,
            shared_state,
        }
    }
}

/// Deliberately only for [`OsRng`], which holds no state
/// Cloning stateful RNG will lead to not really random numbers
impl Clone for LpOutboundPipeline<OsRng> {
    fn clone(&self) -> Self {
        LpOutboundPipeline {
            rng: self.rng,
            nonce: self.nonce,
            debug_config: self.debug_config,
            topology_accessor: self.topology_accessor.clone(),
            shared_state: self.shared_state.clone(),
        }
    }
}

impl<R> Chunking<LpOutboundOptions> for LpOutboundPipeline<R>
where
    R: CryptoRng + Rng,
{
    /// Pad the message and split it into pieces that fill a sphinx payload, each a bare `Fragment`.
    ///
    /// `chunk_size` is what the frame budget leaves for one, so a chunk is exactly what
    /// [`encrypt`](super::routing_security) then builds a packet around. Not
    /// [`pad_and_split_message`](nym_sphinx::preparer::FragmentPreparer::pad_and_split_message),
    /// which takes a [`PacketSize`](nym_sphinx::params::PacketSize) rather than a byte count.
    fn chunked(
        &mut self,
        input: PipelinePayload<LpOutboundOptions>,
        chunk_size: usize,
        timestamp: Instant,
    ) -> Vec<PipelinePayload<LpOutboundOptions>> {
        NymMessage::new_plain(input.data.data)
            .pad_to_full_packet_lengths(chunk_size)
            .split_into_fragments(&mut self.rng, chunk_size)
            .into_iter()
            .map(|fragment| {
                PipelinePayload::new(timestamp, fragment.into_bytes(), input.options, input.dst)
            })
            .collect()
    }
}

// nothing is acknowledged and nothing is covered on this path - see the module docs
impl<R> NoOpReliability for LpOutboundPipeline<R> {}
impl<R> NoOpObfuscation for LpOutboundPipeline<R> {}

impl<R> Framing<LpOutboundOptions> for LpOutboundPipeline<R>
where
    R: CryptoRng + Rng,
{
    type Frame = LpFrame;
    const OVERHEAD_SIZE: usize = LpFrameHeader::SIZE;

    fn to_frame(
        &mut self,
        payload: PipelinePayload<LpOutboundOptions>,
        frame_size: usize,
    ) -> Vec<AddressedTimedData<Self::Frame>> {
        // a chunk the routing stage could not do anything with; it already said why
        if payload.data.data.is_empty() {
            return Vec::new();
        }

        let Ok(frame) = LpFrame::decode(&payload.data.data)
            .inspect_err(|err| warn!("LP outbound: our own frame did not decode back: {err}"))
        else {
            return Vec::new();
        };

        // a sphinx packet is larger than one frame, so it goes out as several - see `nb_frames`
        fragment_lp_message(&mut self.rng, frame, frame_size)
            .into_iter()
            .map(|fragment| {
                AddressedTimedData::new_addressed(
                    payload.data.timestamp,
                    fragment.into_lp_frame(),
                    payload.dst,
                )
            })
            .collect()
    }
}

impl<R> Transport<EncryptedLpPacket> for LpOutboundPipeline<R> {
    type Frame = LpFrame;
    type Error = LpDataHandlerError;
    const OVERHEAD_SIZE: usize = EncryptedLpPacket::OVERHEAD;

    fn to_transport_packet(
        &mut self,
        frame: AddressedTimedData<Self::Frame>,
    ) -> Result<AddressedTimedData<EncryptedLpPacket>, Self::Error> {
        let AddressedTimedData {
            data: TimedData { timestamp, data },
            dst,
            ..
        } = frame;

        let packet = self.shared_state.sessions.prepare(dst, data)?;

        Ok(AddressedTimedData::new_addressed(timestamp, packet, dst))
    }
}

impl<R> WireWrappingPipeline<EncryptedLpPacket, LpOutboundOptions> for LpOutboundPipeline<R>
where
    R: CryptoRng + Rng,
{
    fn packet_size(&self) -> usize {
        MTU
    }
}

impl<R> ClientWrappingPipeline<EncryptedLpPacket, LpOutboundOptions> for LpOutboundPipeline<R> where
    R: CryptoRng + Rng
{
}
