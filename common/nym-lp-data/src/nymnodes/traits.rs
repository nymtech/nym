// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Instant;

use crate::{AddressedTimedData, PipelinePayload, TimedData, TimedPayload};

use crate::common::traits::{Framing, FramingUnwrap};

/// Nymnode processing pipeline: **frames in, frames out**.
///
/// Reassembles an inbound frame into a message, mixes it (decrypt, route, schedule delays),
/// and re-frames the results for their next hops.
///
/// # The transport layer is deliberately outside this trait
///
/// Transport wrap/unwrap — for LP, the session AEAD — is *not* part of the pipeline. The caller
/// decrypts before handing frames in, and encrypts after taking frames out.
///
/// That is not just tidiness. [`mix`] may stamp its output with a future timestamp (a mixnet
/// forward delay), and a caller honouring those delays holds each frame until its time arrives.
/// A transport that allocates per-packet state on wrap — a counter or nonce, as any AEAD does —
/// must therefore wrap at *send* time. Wrapping during processing numbers packets in processing
/// order while transmitting them in delay order, which breaks two things:
///
/// 1. **Replay rejection.** A receiver enforcing a replay window drops anything overtaken by more
///    than the window size while it waited.
/// 2. **The mixing itself.** Such counters are typically *cleartext* on the wire — LP's is, in the
///    outer header — so an observer sees the sequence out of order, and each packet's displacement
///    within it directly encodes the delay that packet was given. That hands back exactly the
///    in-to-out correlation the delay exists to destroy. It is an anonymity bug, not merely a
///    correctness one, and it is the reason this matters even where a receiver is lenient.
///
/// Note the asymmetry: the *inbound* direction carries no such constraint. Unwrapping concurrently
/// changes nothing observable and only has to stay inside the local replay window.
///
/// Keeping transport out means this trait has no opinion on any of it: a stateless transport can
/// wrap whenever it likes, a stateful one waits, and neither is baked into the vocabulary.
///
/// # Type Parameters
/// - `Frame`: frame type consumed and produced. Both framing halves are pinned to it.
///
/// # Associated Types
/// - `Options`: per-message pipeline options carried into the re-framing side.
/// - `MessageKind`: message-kind marker returned by the unwrap side.
///
/// Both are properties of the concrete pipeline rather than something a caller varies, so they
/// live as associated types. This keeps consumers (e.g. a generic worker driver) free of
/// `Options` / `MessageKind` bounds.
///
/// # Required Methods
/// - `mix`: given a reassembled payload and the current timestamp, return zero or more
///   [`PipelinePayload`]s carrying their next-hop addresses.
/// - `frame_size`: budget for outbound frames, i.e. the transport MTU minus whatever the
///   caller's transport layer will add.
///
/// # Provided Methods
/// - `process`: `frame_to_message` → [`mix`] → `to_frame`.
///
/// [`mix`]: NymNodeProcessingPipeline::mix
pub trait NymNodeProcessingPipeline<Frame>:
    FramingUnwrap<<Self as NymNodeProcessingPipeline<Frame>>::MessageKind, Frame = Frame>
    + Framing<<Self as NymNodeProcessingPipeline<Frame>>::Options, Frame = Frame>
{
    type Options;
    type MessageKind;

    /// Size of an outbound frame, including header
    fn frame_size(&self) -> usize;

    fn mix(
        &mut self,
        message_kind: Self::MessageKind,
        payload: TimedPayload,
        timestamp: Instant,
    ) -> Vec<PipelinePayload<Self::Options>>;

    /// Reassemble, mix, and re-frame.
    ///
    /// Returns an empty vec when the inbound frame was a fragment that did not complete a
    /// message, or when `mix` chose to drop it.
    fn process(
        &mut self,
        input: TimedData<Frame>,
        timestamp: Instant,
    ) -> Vec<AddressedTimedData<Frame>> {
        let Some((payload, kind)) = self.frame_to_message(input) else {
            return Vec::new();
        };

        let frame_payload_size = self.frame_size() - <Self as Framing<_>>::OVERHEAD_SIZE;

        self.mix(kind, payload, timestamp)
            .into_iter()
            .flat_map(|mixed| self.to_frame(mixed, frame_payload_size))
            .collect()
    }
}
