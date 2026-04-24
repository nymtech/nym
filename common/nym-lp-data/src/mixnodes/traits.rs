// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{AddressedTimedData, AddressedTimedPayload, TimedData, TimedPayload};

use crate::common::traits::{WireUnwrappingPipeline, WireWrappingPipeline};

// /// Dyn-compatible mirror of [`MixnodeProcessingPipeline`].
// ///
// /// Erases the `Frame` associated type so the pipeline can be stored as
// /// `dyn DynMixnodeProcessingPipeline<Ts, Pkt, NodeId>`.
// ///
// /// Implement [`MixnodeProcessingPipeline`] on your concrete type; the blanket
// /// impl below provides `DynMixnodeProcessingPipeline` for free.  For
// /// pass-through stubs that do not need the full wire layer, you may implement
// /// this trait directly.
// pub trait DynMixnodeProcessingPipeline<Ts, Fr, Pkt, Mk, NodeId> {
//     fn process(
//         &mut self,
//         input: TimedData<Ts, Pkt>,
//         timestamp: Ts,
//     ) -> anyhow::Result<Vec<(NodeId, TimedData<Ts, Pkt>)>>;
// }

// impl<T, Ts, Fr, Pkt, Mk, NodeId> DynMixnodeProcessingPipeline<Ts, Fr, Pkt, Mk, NodeId> for T
// where
//     T: MixnodeProcessingPipeline<Ts, Fr, Pkt, Mk, NodeId>,
//     Ts: Clone,
//     NodeId: Clone,
// {
//     fn process(
//         &mut self,
//         input: TimedData<Ts, Pkt>,
//         timestamp: Ts,
//     ) -> anyhow::Result<Vec<(NodeId, TimedData<Ts, Pkt>)>> {
//         MixnodeProcessingPipeline::process(self, input, timestamp)
//     }
// }

/// Top-level processing trait for a mix node.
///
/// Combines [`WireUnwrappingPipeline`] and [`WireWrappingPipeline`] with a blank [`mix`]
/// step that the implementor fills in (decrypt, route, re-encrypt, cover traffic, etc.).
///
/// # Type Parameters
/// - `Ts`: Timestamp / tick-context type.
/// - `Pkt`: Transport packet type; the same type is consumed and produced.
/// - `NodeId`: Identifier type for the next-hop destination.
///
/// # Associated Types
/// - `Frame`: Intermediate frame type shared by the unwrapping and wrapping wire layers.
///
/// # Required Methods
/// - `mix`: Given a reassembled payload and the current timestamp, return zero or more
///   `(next_hop, payload)` pairs to be re-wrapped and forwarded.
///
/// # Provided Methods
/// - `process`: Unwraps the incoming packet via [`WireUnwrappingPipeline::wire_unwrap`],
///   passes the result to [`mix`], and re-wraps each output payload via
///   [`WireWrappingPipeline::wire_wrap`].
pub trait MixnodeProcessingPipeline<Ts, Fr, Pkt, Mk, NdId>:
    WireUnwrappingPipeline<Ts, Fr, Pkt, Mk> + WireWrappingPipeline<Ts, Fr, Pkt, NdId>
where
    Ts: Clone,
    NdId: Clone,
{
    fn mix(
        &mut self,
        message_kind: Mk,
        payload: TimedPayload<Ts>,
        timestamp: Ts,
    ) -> Vec<AddressedTimedPayload<Ts, NdId>>;

    fn process(
        &mut self,
        input: TimedData<Ts, Pkt>,
        timestamp: Ts,
    ) -> anyhow::Result<Vec<AddressedTimedData<Ts, Pkt, NdId>>> {
        let TimedData {
            data: packet,
            timestamp: ts,
        } = input;
        let Some((payload, kind)) = self.wire_unwrap(packet, ts)? else {
            return Ok(Vec::new());
        };
        let mixed = self.mix(kind, payload, timestamp);
        Ok(mixed
            .into_iter()
            .flat_map(|addressed_data| self.wire_wrap(addressed_data).into_iter())
            .collect())
    }
}
