// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{TimedData, TimedPayload};

use crate::common::traits::{WireUnwrappingPipeline, WireWrappingPipeline};

/// Trait for applying routing security processing (e.g. encryption) to a timed payload.
///
/// # Type Parameters
/// - `Ts`: Timestamp type carried by the `TimedPayload`.
///
/// # Required Methods
/// - `process_routing_security`: Process the routing security mechanism to the given payload,
///   returning a new `TimedPayload` with the processed data.
pub trait RoutingSecurityProcessing<Ts> {
    fn process_routing_security(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts>;
}

/// Dyn-compatible mirror of [`MixnodeProcessingPipeline`].
///
/// Erases the `Frame` associated type so the pipeline can be stored as
/// `dyn DynMixnodeProcessingPipeline<Ts, Pkt, NodeId>`.
///
/// Implement [`MixnodeProcessingPipeline`] on your concrete type; the blanket
/// impl below provides `DynMixnodeProcessingPipeline` for free.  For
/// pass-through stubs that do not need the full wire layer, you may implement
/// this trait directly.
pub trait DynMixnodeProcessingPipeline<Ts, Pkt, NodeId> {
    fn process(
        &mut self,
        input: TimedData<Ts, Pkt>,
        timestamp: Ts,
    ) -> anyhow::Result<Vec<(NodeId, TimedData<Ts, Pkt>)>>;
}

impl<T, Ts, Pkt, NodeId> DynMixnodeProcessingPipeline<Ts, Pkt, NodeId> for T
where
    T: MixnodeProcessingPipeline<Ts, Pkt, NodeId>,
    Ts: Clone,
    NodeId: Clone,
{
    fn process(
        &mut self,
        input: TimedData<Ts, Pkt>,
        timestamp: Ts,
    ) -> anyhow::Result<Vec<(NodeId, TimedData<Ts, Pkt>)>> {
        MixnodeProcessingPipeline::process(self, input, timestamp)
    }
}

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
pub trait MixnodeProcessingPipeline<Ts, Pkt, NodeId>:
    WireUnwrappingPipeline<Ts, Self::Frame, Pkt> + WireWrappingPipeline<Ts, Self::Frame, Pkt>
where
    Ts: Clone,
    NodeId: Clone,
{
    type Frame;

    fn mix(&mut self, payload: TimedPayload<Ts>, timestamp: Ts) -> Vec<(NodeId, TimedPayload<Ts>)>;

    fn process(
        &mut self,
        input: TimedData<Ts, Pkt>,
        timestamp: Ts,
    ) -> anyhow::Result<Vec<(NodeId, TimedData<Ts, Pkt>)>> {
        let TimedData {
            data: packet,
            timestamp: ts,
        } = input;
        let Some((payload, _kind)) = self.wire_unwrap(packet, ts)? else {
            return Ok(Vec::new());
        };
        let mixed = self.mix(payload, timestamp);
        Ok(mixed
            .into_iter()
            .flat_map(|(node_id, out_payload)| {
                self.wire_wrap(out_payload)
                    .into_iter()
                    .map(move |pkt| (node_id.clone(), pkt))
            })
            .collect())
    }
}
