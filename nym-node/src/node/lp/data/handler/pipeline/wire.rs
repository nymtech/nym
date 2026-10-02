// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The wire layer, in two halves.
//!
//! [`FramingPipeline`] fragments outbound messages and reassembles inbound ones. Each pipeline owns
//! one, since reassembly is stateful and fragmentation needs an rng.
//!
//! [`LpTransport`] encodes and decodes LP packets. It holds nothing - both directions are a
//! function of [`SharedLpDataState`] and the packet - so it is a unit struct with associated
//! functions, callable from anywhere that has the state to hand. That matters for the outbound
//! direction: the wrap is applied by the data handler at release time, and the handler owns no
//! pipeline.
//!
//! Neither half knows the application message type carried in the frame.

use std::{sync::Arc, time::Instant};

use nym_lp_data::{
    AddressedTimedData, TimedData,
    fragmentation::fragment::fragment_lp_message,
    packet::{EncryptedLpPacket, LpFrame, frame::LpFrameKind},
};
use rand::Rng;
use tracing::{trace, warn};

use crate::node::lp::data::shared::SharedLpDataState;
use crate::node::lp::error::LpHandlerError;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;

/// Framing layer: fragmentation outbound, reassembly inbound.
pub struct FramingPipeline<R> {
    state: Arc<SharedLpDataState>,
    rng: R,
}

impl<R: Rng> FramingPipeline<R> {
    pub fn new(state: Arc<SharedLpDataState>, rng: R) -> Self {
        Self { state, rng }
    }

    /// Wrap an [`LpFrame`] into one or more addressed frames, fragmenting it if its content would
    /// not fit in a single frame's payload.
    ///
    /// `frame_payload_size` is what a frame can *carry*, with its own header already accounted
    /// for - comparing the serialised length against it would charge for the header twice.
    pub fn message_to_frame<NdId: Copy>(
        &mut self,
        timestamp: Instant,
        frame: LpFrame,
        dst: NdId,
        frame_payload_size: usize,
    ) -> Vec<AddressedTimedData<LpFrame, NdId>> {
        let output_frames = if frame.content.len() > frame_payload_size {
            fragment_lp_message(&mut self.rng, frame, frame_payload_size)
                .into_iter()
                .map(|f| f.into_lp_frame())
                .collect()
        } else {
            vec![frame]
        };

        output_frames
            .into_iter()
            .map(|f| AddressedTimedData::new_addressed(timestamp, f, dst))
            .collect()
    }

    /// If the frame carries a fragment, attempt reassembly; otherwise return the frame as-is.
    ///
    /// Returns `None` when more fragments are needed or reassembly fails. The returned frame is
    /// guaranteed not to be a fragment.
    pub fn frame_to_maybe_message(
        &mut self,
        frame: TimedData<LpFrame>,
    ) -> Option<TimedData<LpFrame>> {
        let reassembled = if frame.data.kind() == LpFrameKind::FragmentedData {
            let fragment = frame
                .data
                .try_into()
                .inspect_err(|e| {
                    tracing::error!("Failed to recover a fragment : {e}");
                    self.state.malformed_packet();
                })
                .ok()?;
            let message = self
                .state
                .message_reconstructor
                .insert_new_fragment(fragment, frame.timestamp)?
                .inspect_err(|e| {
                    tracing::error!("Failed to recover a frame : {e}");
                    self.state.malformed_packet();
                })
                .ok()?;
            TimedData::new(frame.timestamp, message)
        } else {
            frame
        };

        if reassembled.data.kind() == LpFrameKind::FragmentedData {
            warn!(
                "Fragmented data inside fragmented data, it shouldn't happen. Dropping the message"
            );
            None
        } else {
            Some(reassembled)
        }
    }
}

/// Transport layer: LP packet encode/decode.
pub struct LpTransport;

impl LpTransport {
    /// Encrypt an [`LpFrame`] into an [`EncryptedLpPacket`] for the wire, on the session currently
    /// sending to the frame's destination.
    ///
    /// Applied at *release* time rather than during processing: the counter the session assigns is
    /// cleartext in the outer header, so it must be allocated in send order. See
    /// [`LpDataHandler::wrap_due_frames`](crate::node::lp::data::handler::LpDataHandler).
    ///
    /// The frame is consumed either way, so callers that want to keep it - to park it until a
    /// session exists - must check for one before calling. An error here is a genuine failure of an
    /// existing session, not an absent one.
    ///
    /// The frame arrives addressed by *identity* and leaves addressed by a [`std::net::SocketAddr`]. Both
    /// come out of the same lookup: a node is its own address, while a client is wherever it was
    /// last seen - resolved here, at release time, so a client that has moved since the frame was
    /// queued is still reached.
    pub fn frame_to_packet(
        state: &SharedLpDataState,
        frame: AddressedTimedData<LpFrame, NymNodeRoutingAddress>,
    ) -> Result<AddressedTimedData<EncryptedLpPacket>, LpHandlerError> {
        let AddressedTimedData {
            data: TimedData { timestamp, data },
            dst,
            ..
        } = frame;

        let (packet, wire_dst) = state.send_frame(dst, data)?;

        Ok(AddressedTimedData::new_addressed(
            timestamp, packet, wire_dst,
        ))
    }

    /// Decrypt an [`EncryptedLpPacket`] off the wire into an [`LpFrame`], on the session its outer
    /// header names.
    ///
    /// The session also validates the counter against its replay window and marks it, so a replayed
    /// or long-delayed packet is rejected here rather than downstream.
    pub fn packet_to_frame(
        state: &SharedLpDataState,
        packet: EncryptedLpPacket,
        timestamp: Instant,
    ) -> Result<TimedData<LpFrame>, LpHandlerError> {
        let receiver_index = packet.outer_header().receiver_idx;

        let frame = state.receive_packet(packet).inspect_err(|_| {
            // An index naming no session is the signature of a peer that restarted and lost its
            // half of the pairing: it is still sending on a session this node no longer holds.
            // Distinct from a decrypt failure, and the trigger for redialling that peer.
            trace!("LP transport: no session for receiver index {receiver_index}");
        })?;

        Ok(TimedData {
            timestamp,
            data: frame,
        })
    }
}
