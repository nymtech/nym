// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Hiding who is talking to whom: one chunk becomes a sphinx packet, framed for the gateway to
//! forward.
//!
//! The route is drawn per chunk, so the fragments of one message do not travel together. Drawing it
//! and building the packet is [`FragmentPreparer`]'s job, which the pipeline implements below by
//! answering what the sphinx layer needs from the client's config. The pipeline *is* the preparer,
//! which is what lets this stage share
//! [`prepare_chunk_for_lp`](FragmentPreparer::prepare_chunk_for_lp) with the classic path rather
//! than writing route selection twice.

use std::time::Duration;

use nym_lp_data::PipelinePayload;
use nym_lp_data::clients::traits::RoutingSecurity;
use nym_lp_data::packet::FRAMES_PER_SPHINX_PACKET;
use nym_lp_data::packet::frame::LpFrameHeader;
use nym_sphinx::preparer::FragmentPreparer;
use rand::{CryptoRng, Rng};
use tracing::warn;

use super::{LpOutboundOptions, LpOutboundPipeline};

impl<R> RoutingSecurity<LpOutboundOptions> for LpOutboundPipeline<R>
where
    R: CryptoRng + Rng,
{
    // the sphinx packet, plus the frame header naming the hop it is forwarded to
    const OVERHEAD_SIZE: usize =
        nym_sphinx::HEADER_SIZE + nym_sphinx::PAYLOAD_OVERHEAD_SIZE + LpFrameHeader::SIZE;

    fn nb_frames(&self) -> usize {
        FRAMES_PER_SPHINX_PACKET
    }

    /// Wrap one chunk in a sphinx packet ending at the recipient, and frame it for the gateway.
    ///
    /// A chunk that cannot be routed comes out empty: the stage is infallible by contract, so an
    /// empty payload is how it says nothing came of this one, and
    /// [`to_frame`](nym_lp_data::common::traits::Framing::to_frame) drops it.
    fn encrypt(
        &mut self,
        input: PipelinePayload<LpOutboundOptions>,
    ) -> PipelinePayload<LpOutboundOptions> {
        let empty = |input: PipelinePayload<LpOutboundOptions>| {
            PipelinePayload::new(input.data.timestamp, Vec::new(), input.options, input.dst)
        };

        let Ok(fragment) =
            nym_sphinx::chunking::fragment::Fragment::try_from_bytes(&input.data.data)
                .inspect_err(|err| warn!("LP outbound: our own chunk did not parse back: {err}"))
        else {
            return empty(input);
        };

        let Some(topology) = self.topology_accessor.current_route_provider() else {
            warn!("LP outbound: no topology to route against yet");
            return empty(input);
        };

        // a fresh route per fragment, so they do not travel together
        let Ok(prepared) = self
            .prepare_chunk_for_lp(fragment, &topology, &input.options.recipient)
            .inspect_err(|err| {
                warn!(
                    "LP outbound: no route to {}: {err}",
                    input.options.recipient
                )
            })
        else {
            return empty(input);
        };

        let Ok(frame) = prepared
            .into_lp_frame()
            .inspect_err(|err| warn!("LP outbound: malformed sphinx packet: {err}"))
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

/// What the sphinx layer needs to know, answered from the client's config.
impl<R> FragmentPreparer for LpOutboundPipeline<R>
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

    /// Nothing on this path is SURB-acknowledged, so no ack is ever built with this - see the
    /// [module docs](super).
    fn average_ack_delay(&self) -> Duration {
        self.debug_config.acknowledgements.average_ack_delay
    }
}
