// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use strum::{AsRefStr, EnumIter, EnumProperty, IntoEnumIterator};
use tokio::time::Instant;

/// Histogram buckets (seconds) for per-stage and total packet latency: exponential, ~100us .. ~6.5s.
/// Shared by every latency stage so the waterfall is directly comparable; the top finite bucket is
/// intentionally high so a rare multi-second processing spike is measured with magnitude rather than
/// being clipped into the `+Inf` overflow.
const STAGE_LATENCY_BUCKETS: [f64; 17] = [
    0.0001, 0.0002, 0.0004, 0.0008, 0.0016, 0.0032, 0.0064, 0.0128, 0.0256, 0.0512, 0.1024, 0.2048,
    0.4096, 0.8192, 1.6384, 3.2768, 6.5536,
];

/// Count buckets (1 .. MAX_DRAIN_BATCH) for the forwarder drain-batch-size histogram.
const DRAIN_BATCH_BUCKETS: [f64; 9] = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0];

/// Fill-ratio buckets (used/capacity) for the per-connection egress buffer. A ratio near 1.0 means
/// the buffer is close to full and packets to that peer are about to be dropped.
const EGRESS_FILL_BUCKETS: [f64; 9] = [0.05, 0.1, 0.25, 0.5, 0.75, 0.9, 0.95, 0.99, 1.0];

/// Every histogram this crate emits, defined in one place. `AsRefStr` (`#[strum(to_string=...)]`)
/// gives the prometheus metric name - the bare `mixnet_packet_*` family, with no per-crate prefix
/// since this is a shared library writing straight to the process-global registry. The `help` prop
/// gives the description and [`MixnetMetric::buckets`] gives the bucket layout.
///
/// Register the whole family at boot with [`register_all`]. Latency-stage variants are observed via
/// the [`PacketTrace`] stopwatch; the auxiliary variants via the `observe_*` helpers. (Passing an
/// auxiliary variant to `PacketTrace::record` is meaningless but harmless.)
#[derive(Clone, Copy, EnumIter, AsRefStr, EnumProperty)]
pub enum MixnetMetric {
    // ----- latency stages: the per-packet waterfall, recorded via `PacketTrace` -----
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

    // ----- auxiliary histograms: observed directly, not part of the latency waterfall -----
    /// number of packets the forwarder drained from the ingress channel per wakeup
    #[strum(to_string = "mixnet_packet_forwarder_drain_batch_size")]
    #[strum(props(
        help = "Number of ingress packets the forwarder drained per select! wakeup (batch size)"
    ))]
    ForwarderDrainBatchSize,
    /// number of expired packets the forwarder drained from the delay queue per wakeup
    #[strum(to_string = "mixnet_packet_forwarder_delay_drain_batch_size")]
    #[strum(props(
        help = "Number of expired delay-queue packets the forwarder drained per select! wakeup (batch size)"
    ))]
    ForwarderDelayDrainBatchSize,
    /// per-connection egress buffer occupancy (used/capacity) at send time
    #[strum(to_string = "mixnet_packet_egress_buffer_fill_ratio")]
    #[strum(props(
        help = "Per-connection egress buffer fill ratio (used/capacity) sampled at packet send time"
    ))]
    EgressBufferFillRatio,
}

impl MixnetMetric {
    /// Histogram bucket layout for this metric.
    fn buckets(&self) -> &'static [f64] {
        match self {
            MixnetMetric::ForwarderDrainBatchSize | MixnetMetric::ForwarderDelayDrainBatchSize => {
                &DRAIN_BATCH_BUCKETS
            }
            MixnetMetric::EgressBufferFillRatio => &EGRESS_FILL_BUCKETS,
            // every latency stage shares the seconds buckets
            _ => &STAGE_LATENCY_BUCKETS,
        }
    }
}

/// Pre-register every histogram (at zero) into the global metrics registry so the whole
/// `mixnet_packet_*` family is present on the prometheus endpoint from boot, before anything has
/// been observed. Idempotent.
pub fn register_all() {
    let registry = nym_metrics::metrics_registry();
    for metric in MixnetMetric::iter() {
        registry.register_histogram(
            metric.as_ref(),
            metric.get_str("help"),
            Some(metric.buckets()),
        );
    }
}

/// Observe a value into a metric's histogram in the process-global registry.
fn observe(metric: MixnetMetric, value: f64) {
    nym_metrics::metrics_registry().maybe_register_and_add_to_histogram(
        metric.as_ref(),
        value,
        Some(metric.buckets()),
        metric.get_str("help"),
    );
}

/// Observe how many ingress-channel packets the forwarder drained in a single wakeup.
pub fn observe_drain_batch_size(batch_size: usize) {
    observe(MixnetMetric::ForwarderDrainBatchSize, batch_size as f64);
}

/// Observe how many expired delay-queue packets the forwarder drained in a single wakeup.
pub fn observe_delay_drain_batch_size(batch_size: usize) {
    observe(
        MixnetMetric::ForwarderDelayDrainBatchSize,
        batch_size as f64,
    );
}

/// Observe how full a per-connection egress buffer was when a packet was queued for it.
pub fn observe_egress_buffer_fill(used: usize, capacity: usize) {
    if capacity == 0 {
        return;
    }
    observe(
        MixnetMetric::EgressBufferFillRatio,
        used as f64 / capacity as f64,
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

    /// Whether this packet is being traced (sampled).
    pub fn is_sampled(&self) -> bool {
        matches!(self, PacketTrace::On { .. })
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
    pub fn record(&mut self, stage: MixnetMetric) {
        if let Some(secs) = self.lap() {
            observe(stage, secs);
        }
    }

    /// Observe the end-to-end [`MixnetMetric::Total`] latency (since receive) if sampled. Unlike
    /// [`PacketTrace::record`] this does not lap, so it can be called at the very end.
    pub fn record_total(&self) {
        if let Some(secs) = self.total() {
            observe(MixnetMetric::Total, secs);
        }
    }

    /// Observe an explicit `secs` value for `stage` if the packet is sampled, without lapping the
    /// stage cursor. For diagnostics that don't fit the sequential waterfall (e.g. delay-queue
    /// overrun, measured against the target deadline rather than the previous stage).
    pub fn record_value(&self, stage: MixnetMetric, secs: f64) {
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
    pub fn record(&mut self, stage: MixnetMetric) {
        self.trace.record(stage)
    }

    /// Observe an explicit value for the carried trace (see [`PacketTrace::record_value`]).
    pub fn record_value(&self, stage: MixnetMetric, secs: f64) {
        self.trace.record_value(stage, secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // guards that AsRefStr honours `#[strum(to_string = ...)]` (rather than falling back to the
    // variant name), that every metric is in the `mixnet_packet_*` family, and carries a help
    // string, and that each metric resolves to a bucket layout.
    #[test]
    fn every_metric_has_a_mixnet_packet_name_help_and_buckets() {
        for metric in MixnetMetric::iter() {
            assert!(
                metric.as_ref().starts_with("mixnet_packet_"),
                "unexpected metric name: {}",
                metric.as_ref()
            );
            assert!(
                metric.get_str("help").is_some(),
                "missing help for {}",
                metric.as_ref()
            );
            assert!(
                !metric.buckets().is_empty(),
                "missing buckets for {}",
                metric.as_ref()
            );
        }
        assert_eq!(
            MixnetMetric::Unwrap.as_ref(),
            "mixnet_packet_stage_unwrap_seconds"
        );
        assert_eq!(
            MixnetMetric::Total.as_ref(),
            "mixnet_packet_total_latency_seconds"
        );
        assert_eq!(
            MixnetMetric::ForwarderDrainBatchSize.as_ref(),
            "mixnet_packet_forwarder_drain_batch_size"
        );
    }
}
