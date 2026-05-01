// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{AddressedTimedData, PipelinePayload, TimedPayload};

use crate::common::traits::{WireUnwrappingPipeline, WireWrappingPipeline};

/// Top-level processing trait for a mix node.
///
/// Combines [`WireUnwrappingPipeline`] and [`WireWrappingPipeline`] with a blank [`mix`]
/// step that the implementor fills in (decrypt, route, re-encrypt, cover traffic, etc.).
///
/// # Type Parameters
/// - `Ts`: Timestamp / tick-context type.
/// - `Pkt`: Transport packet type; the same type is consumed and produced.
/// - `Opts`: Per-message pipeline options carried into the re-wrapping side.
/// - `Mk`: Message-kind marker returned by the unwrap side.
/// - `NdId`: Identifier type for the next-hop destination.
///
/// Frame types are owned by the wire sub-traits as associated items and do not
/// appear in this trait's parameter list.
///
/// # Required Methods
/// - `mix`: Given a reassembled payload and the current timestamp, return zero or more
///   [`PipelinePayload`]s carrying their next-hop addresses to be re-wrapped and forwarded.
///
/// # Provided Methods
/// - `process`: Unwraps the incoming packet via [`WireUnwrappingPipeline::wire_unwrap`],
///   passes the result to [`mix`], and re-wraps each output payload via
///   [`WireWrappingPipeline::wire_wrap`].
///
/// [`mix`]: MixnodeProcessingPipeline::mix
pub trait MixnodeProcessingPipeline<Ts, Pkt, Opts, Mk, NdId>:
    WireUnwrappingPipeline<Ts, Pkt, Mk> + WireWrappingPipeline<Ts, Pkt, Opts, NdId>
where
    Ts: Clone,
    NdId: Clone,
{
    fn mix(
        &mut self,
        message_kind: Mk,
        payload: TimedPayload<Ts>,
        timestamp: Ts,
    ) -> Vec<PipelinePayload<Ts, Opts, NdId>>;

    fn process(
        &mut self,
        input: Pkt,
        timestamp: Ts,
    ) -> Result<Vec<AddressedTimedData<Ts, Pkt, NdId>>, Self::Error> {
        let Some((payload, kind)) = self.wire_unwrap(input, timestamp.clone())? else {
            return Ok(Vec::new());
        };
        let mixed = self.mix(kind, payload, timestamp);
        Ok(mixed
            .into_iter()
            .flat_map(|addressed_data| self.wire_wrap(addressed_data).into_iter())
            .collect())
    }
}
