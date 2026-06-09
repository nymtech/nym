// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use strum::{AsRefStr, EnumIter, EnumProperty, IntoEnumIterator};
use tokio::time::Instant;

/// Histogram buckets (seconds) for per-stage and total packet latency: exponential,
/// ~100us .. ~1.6s. Shared by every stage so the waterfall is directly comparable.
const STAGE_LATENCY_BUCKETS: [f64; 14] = [
    0.0001, 0.0002, 0.0004, 0.0008, 0.0016, 0.0032, 0.0064, 0.0128, 0.0256, 0.0512, 0.1024, 0.2048,
    0.4096, 0.8192,
];

/// A stage in the packet-forwarding pipeline, in order. Each maps to its own latency histogram
/// (`AsRefStr` = metric name, `help` prop = description); `Total` is the end-to-end
/// receive -> socket-write time. Defined here so call sites just name the stage.
#[derive(Clone, Copy, EnumIter, AsRefStr, EnumProperty)]
pub enum TraceStage {
    /// receive -> sphinx unwrap (partial: shared secret + header MAC)
    #[strum(to_string = "mixnet_packet_stage_unwrap_seconds")]
    #[strum(props(help = "Seconds spent unwrapping a received sphinx packet"))]
    Unwrap,
    /// unwrap -> replay-check + finalise (includes the deferral wait)
    #[strum(to_string = "mixnet_packet_stage_replay_check_seconds")]
    #[strum(props(
        help = "Seconds from partial-unwrap to replay-check + finalise (includes the deferral wait)"
    ))]
    ReplayCheck,
    /// wait in the ingress -> forwarder channel
    #[strum(to_string = "mixnet_packet_stage_forwarder_queue_seconds")]
    #[strum(props(
        help = "Seconds a forwarded packet waited in the ingress-to-forwarder channel"
    ))]
    ForwarderQueue,
    /// the (intended) mix delay
    #[strum(to_string = "mixnet_packet_stage_delay_queue_seconds")]
    #[strum(props(help = "Seconds a forwarded packet spent in the (intended) mix delay queue"))]
    DelayQueue,
    /// diagnostic overlay on `DelayQueue`: how late beyond the target release the packet was
    /// actually forwarded (delay-queue scheduling/retrieval overhead, measured vs the deadline)
    #[strum(to_string = "mixnet_packet_stage_delay_queue_overrun_seconds")]
    #[strum(props(
        help = "Seconds a delayed packet was forwarded beyond its target release time (delay-queue scheduling/retrieval overhead)"
    ))]
    DelayQueueOverrun,
    /// wait in the per-connection egress buffer
    #[strum(to_string = "mixnet_packet_stage_egress_queue_seconds")]
    #[strum(props(
        help = "Seconds a forwarded packet waited in the per-connection egress buffer"
    ))]
    EgressQueue,
    /// flushing the packet batch to the socket
    #[strum(to_string = "mixnet_packet_stage_socket_write_seconds")]
    #[strum(props(help = "Seconds spent flushing a forwarded packet batch to the socket"))]
    SocketWrite,
    /// end-to-end: receive -> socket write
    #[strum(to_string = "mixnet_packet_total_latency_seconds")]
    #[strum(props(help = "Total in-node latency of a forwarded packet, receive to socket write"))]
    Total,
}

/// Pre-register every stage histogram (at zero) into the global metrics registry so the whole
/// `mixnet_packet_*` family is present on the prometheus endpoint from boot, before any sampled
/// packet has been observed. Idempotent.
pub fn register_stage_metrics() {
    let registry = nym_metrics::metrics_registry();
    for stage in TraceStage::iter() {
        registry.register_histogram(
            stage.as_ref(),
            stage.get_str("help"),
            Some(STAGE_LATENCY_BUCKETS.as_slice()),
        );
    }
}

/// Observe a stage latency into the process-global metrics registry. Explicit metric name (no
/// per-crate prefix) so every stage lands in one uniform `mixnet_packet_*` family regardless of
/// which crate records it.
fn observe(stage: TraceStage, secs: f64) {
    nym_metrics::metrics_registry().maybe_register_and_add_to_histogram(
        stage.as_ref(),
        secs,
        Some(STAGE_LATENCY_BUCKETS.as_slice()),
        stage.get_str("help"),
    );
}

/// A lightweight per-packet stopwatch for attributing forwarding latency to pipeline
/// stages. Unsampled packets carry the `Off` variant and do zero clock reads, so the only
/// cost on the hot path is moving a small `Copy` value and a branch.
#[derive(Clone, Copy)]
pub enum PacketTrace {
    Off,
    On {
        received_at: Instant,
        stage_at: Instant,
    },
}

impl PacketTrace {
    /// Begin tracing. Reads the clock only for sampled packets.
    pub fn start(sampled: bool) -> Self {
        if sampled {
            let now = Instant::now();
            PacketTrace::On {
                received_at: now,
                stage_at: now,
            }
        } else {
            PacketTrace::Off
        }
    }

    /// Seconds spent in the stage just completed, advancing the cursor to now.
    /// Returns `None` for unsampled packets.
    fn lap(&mut self) -> Option<f64> {
        match self {
            PacketTrace::Off => None,
            PacketTrace::On { stage_at, .. } => {
                let now = Instant::now();
                let secs = now.duration_since(*stage_at).as_secs_f64();
                *stage_at = now;
                Some(secs)
            }
        }
    }

    /// Seconds since tracing began (i.e. since the packet was received), or `None` if unsampled.
    fn total(&self) -> Option<f64> {
        match self {
            PacketTrace::Off => None,
            PacketTrace::On { received_at, .. } => {
                Some(Instant::now().duration_since(*received_at).as_secs_f64())
            }
        }
    }

    /// Close out the stage just completed: lap the timer and, only if the packet is sampled,
    /// observe `stage`'s latency histogram.
    pub fn record(&mut self, stage: TraceStage) {
        if let Some(secs) = self.lap() {
            observe(stage, secs);
        }
    }

    /// Observe the end-to-end [`TraceStage::Total`] latency (since receive) if sampled. Unlike
    /// [`PacketTrace::record`] this does not lap, so it can be called at the very end.
    pub fn record_total(&self) {
        if let Some(secs) = self.total() {
            observe(TraceStage::Total, secs);
        }
    }

    /// Observe an explicit `secs` value for `stage` if the packet is sampled, without lapping the
    /// stage cursor. For diagnostics that don't fit the sequential waterfall (e.g. delay-queue
    /// overrun, measured against the target deadline rather than the previous stage).
    pub fn record_value(&self, stage: TraceStage, secs: f64) {
        if matches!(self, PacketTrace::On { .. }) {
            observe(stage, secs);
        }
    }
}

/// A value paired with its in-flight latency trace, so the trace rides along as the value is
/// moved between pipeline stages (and transformed via [`Traced::map`]). Used wherever a packet
/// crosses a queue/channel: replay batch, delay queue, egress channel.
pub struct Traced<T> {
    pub inner: T,
    pub trace: PacketTrace,
}

impl<T> Traced<T> {
    pub fn new(inner: T, trace: PacketTrace) -> Self {
        Traced { inner, trace }
    }

    /// Transform the carried value, keeping the same trace.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Traced<U> {
        Traced {
            inner: f(self.inner),
            trace: self.trace,
        }
    }

    /// Record the stage just completed for the carried trace (see [`PacketTrace::record`]).
    pub fn record(&mut self, stage: TraceStage) {
        self.trace.record(stage)
    }

    /// Observe an explicit value for the carried trace (see [`PacketTrace::record_value`]).
    pub fn record_value(&self, stage: TraceStage, secs: f64) {
        self.trace.record_value(stage, secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // guards that AsRefStr honours `#[strum(to_string = ...)]` (rather than falling back to the
    // variant name) and that every stage carries a help string.
    #[test]
    fn every_stage_has_a_mixnet_packet_name_and_help() {
        for stage in TraceStage::iter() {
            assert!(
                stage.as_ref().starts_with("mixnet_packet_"),
                "unexpected metric name: {}",
                stage.as_ref()
            );
            assert!(
                stage.get_str("help").is_some(),
                "missing help for {}",
                stage.as_ref()
            );
        }
        assert_eq!(
            TraceStage::Unwrap.as_ref(),
            "mixnet_packet_stage_unwrap_seconds"
        );
        assert_eq!(
            TraceStage::Total.as_ref(),
            "mixnet_packet_total_latency_seconds"
        );
    }
}
