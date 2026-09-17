// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Turning what a provider wants to send into frames for the gateway hosting it.
//!
//! The same six stages every client wrapping pipeline has, and the same *processing* the client's
//! own outbound pipeline calls - route selection and sphinx are shared, not reimplemented. What
//! differs is the bottom two stages, and only because this pipeline has no wire under it:
//!
//! - **framing** produces the frame whole. A client splits a ~2KB sphinx packet across two frames
//!   because a frame has to fit an MTU; a provider hands its frame to a gateway in the same process
//!   over a channel, so there is no MTU to fit and nothing to split. The gateway fragments on
//!   egress, because the gateway is what meets the wire.
//! - **transport** does nothing at all. Transport here means the LP session's AEAD, and there is no
//!   session: both halves are one process, so there is no one to hide the frame from.
//!
//! What comes out is exactly what a gateway holds after decrypting a real client's packet and
//! putting its fragments back together, which is why the far end needs no special case.

use std::time::{Duration, Instant};

use nym_client_core::client::lp::data::handler::pipeline::outbound::LpOutboundOptions;
use nym_client_core::client::topology_control::TopologyAccessor;
use nym_client_core::config::DebugConfig;
use nym_lp_data::clients::helpers::{NoOpObfuscation, NoOpReliability};
use nym_lp_data::clients::traits::{Chunking, ClientWrappingPipeline, RoutingSecurity};
use nym_lp_data::common::traits::{Framing, Transport, WireWrappingPipeline};
use nym_lp_data::packet::frame::LpFrameHeader;
use nym_lp_data::packet::{LpFrame, FRAMES_PER_SPHINX_PACKET, MAX_FRAME_PAYLOAD_SIZE};
use nym_lp_data::{AddressedTimedData, PipelinePayload};
use nym_sphinx::chunking::fragment::Fragment;
use nym_sphinx::message::NymMessage;
use nym_sphinx::preparer::FragmentPreparer;
use rand::{CryptoRng, Rng};
use tracing::warn;

use crate::lp::error::LpProviderError;

/// Wraps outbound messages for a provider on the LP path.
///
/// Synchronous throughout, like its client counterpart: the topology accessor is wait-free to read,
/// so no stage here needs the runtime or a lock.
pub struct SpOutboundPipeline<R> {
    /// Draws routes and sphinx delays.
    rng: R,

    /// Fixes the route drawn for a given fragment, when the config asks for it.
    nonce: i32,

    /// What the sphinx layer is built with; read through [`FragmentPreparer`].
    debug_config: DebugConfig,

    /// Where routes come from. Read per fragment, so each takes its own path.
    topology_accessor: TopologyAccessor,
}

impl<R> SpOutboundPipeline<R> {
    pub fn new(rng: R, debug_config: DebugConfig, topology_accessor: TopologyAccessor) -> Self {
        SpOutboundPipeline {
            rng,
            nonce: 0,
            debug_config,
            topology_accessor,
        }
    }
}

impl<R: Clone> Clone for SpOutboundPipeline<R> {
    fn clone(&self) -> Self {
        SpOutboundPipeline {
            rng: self.rng.clone(),
            nonce: self.nonce,
            debug_config: self.debug_config,
            topology_accessor: self.topology_accessor.clone(),
        }
    }
}

impl<R> Chunking<LpOutboundOptions> for SpOutboundPipeline<R>
where
    R: CryptoRng + Rng,
{
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

// nothing is acknowledged and nothing is covered on this path, as on the client's
impl<R> NoOpReliability for SpOutboundPipeline<R> {}
impl<R> NoOpObfuscation for SpOutboundPipeline<R> {}

impl<R> RoutingSecurity<LpOutboundOptions> for SpOutboundPipeline<R>
where
    R: CryptoRng + Rng,
{
    // the sphinx packet, plus the frame header naming the hop it is forwarded to
    const OVERHEAD_SIZE: usize =
        nym_sphinx::HEADER_SIZE + nym_sphinx::PAYLOAD_OVERHEAD_SIZE + LpFrameHeader::SIZE;

    /// The same as a client's, which together with the frame budget is what makes the chunking
    /// identical - see the note on [`packet_size`].
    ///
    /// [`packet_size`]: WireWrappingPipeline::packet_size
    fn nb_frames(&self) -> usize {
        FRAMES_PER_SPHINX_PACKET
    }

    /// Wrap one chunk in a sphinx packet ending at the recipient, and frame it for the gateway.
    ///
    /// A chunk that cannot be routed comes out empty: the stage is infallible by contract, so an
    /// empty payload is how it says nothing came of this one, and [`to_frame`](Framing::to_frame)
    /// drops it.
    fn encrypt(
        &mut self,
        input: PipelinePayload<LpOutboundOptions>,
    ) -> PipelinePayload<LpOutboundOptions> {
        let empty = |input: PipelinePayload<LpOutboundOptions>| {
            PipelinePayload::new(input.data.timestamp, Vec::new(), input.options, input.dst)
        };

        let Ok(fragment) = Fragment::try_from_bytes(&input.data.data).inspect_err(|err| {
            warn!("LP provider outbound: our own chunk did not parse back: {err}")
        }) else {
            return empty(input);
        };

        let Some(topology) = self.topology_accessor.current_route_provider() else {
            warn!("LP provider outbound: no topology to route against yet");
            return empty(input);
        };

        // a fresh route per fragment, so they do not travel together
        let Ok(prepared) = self
            .prepare_chunk_for_lp(fragment, &topology, &input.options.recipient)
            .inspect_err(|err| {
                warn!(
                    "LP provider outbound: no route to {}: {err}",
                    input.options.recipient
                )
            })
        else {
            return empty(input);
        };

        let Ok(frame) = prepared
            .into_lp_frame()
            .inspect_err(|err| warn!("LP provider outbound: malformed sphinx packet: {err}"))
        else {
            return empty(input);
        };

        PipelinePayload::new(
            input.data.timestamp,
            frame.to_bytes(),
            input.options,
            input.dst,
        )
    }
}

/// What the sphinx layer needs to know, answered from the provider's config.
impl<R> FragmentPreparer for SpOutboundPipeline<R>
where
    R: CryptoRng + Rng,
{
    type Rng = R;

    fn use_legacy_sphinx_format(&self) -> bool {
        self.debug_config.traffic.use_legacy_sphinx_format
    }

    fn mix_hops_disabled(&self) -> bool {
        self.debug_config.traffic.disable_mix_hops
    }

    fn deterministic_route_selection(&self) -> bool {
        self.debug_config.traffic.deterministic_route_selection
    }

    fn rng(&mut self) -> &mut Self::Rng {
        &mut self.rng
    }

    fn nonce(&self) -> i32 {
        self.nonce
    }

    fn average_packet_delay(&self) -> Duration {
        self.debug_config.traffic.average_packet_delay
    }

    /// Nothing on this path is SURB-acknowledged, so no ack is ever built with this.
    fn average_ack_delay(&self) -> Duration {
        self.debug_config.acknowledgements.average_ack_delay
    }
}

impl<R> Framing<LpOutboundOptions> for SpOutboundPipeline<R>
where
    R: CryptoRng + Rng,
{
    type Frame = LpFrame;

    /// Zero, and not because framing is free: the frame header was already paid for by
    /// [`RoutingSecurity::OVERHEAD_SIZE`], which is where the frame is actually built. Charging it
    /// again here would shrink every chunk by a header.
    const OVERHEAD_SIZE: usize = 0;

    /// One frame, whole. See the module docs for why there is nothing to fragment against.
    fn to_frame(
        &mut self,
        payload: PipelinePayload<LpOutboundOptions>,
        _frame_size: usize,
    ) -> Vec<AddressedTimedData<Self::Frame>> {
        // a chunk the routing stage could not do anything with; it already said why
        if payload.data.data.is_empty() {
            return Vec::new();
        }

        let Ok(frame) = LpFrame::decode(&payload.data.data).inspect_err(|err| {
            warn!("LP provider outbound: our own frame did not decode back: {err}")
        }) else {
            return Vec::new();
        };

        vec![AddressedTimedData::new_addressed(
            payload.data.timestamp,
            frame,
            payload.dst,
        )]
    }
}

/// Nothing: there is no session to encrypt on, and the frame is already what the gateway wants.
impl<R> Transport<LpFrame> for SpOutboundPipeline<R> {
    type Frame = LpFrame;
    type Error = LpProviderError;
    const OVERHEAD_SIZE: usize = 0;

    fn to_transport_packet(
        &mut self,
        frame: AddressedTimedData<Self::Frame>,
    ) -> Result<AddressedTimedData<LpFrame>, Self::Error> {
        Ok(frame)
    }
}

impl<R> WireWrappingPipeline<LpFrame, LpOutboundOptions> for SpOutboundPipeline<R>
where
    R: CryptoRng + Rng,
{
    /// A frame's payload budget, not a datagram's, and that is the whole point.
    ///
    /// With both wire overheads at zero this is what `frame_size()` comes out as, and `chunk_size()`
    /// derives from that - so anything else here would split messages differently from every real
    /// client, and the difference would be visible in the size of the sphinx packets leaving the
    /// node. Same constant, same chunking, same packets.
    fn packet_size(&self) -> usize {
        MAX_FRAME_PAYLOAD_SIZE
    }
}

impl<R> ClientWrappingPipeline<LpFrame, LpOutboundOptions> for SpOutboundPipeline<R> where
    R: CryptoRng + Rng
{
}
