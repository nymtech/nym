// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Samples tokio runtime metrics (scheduling pressure, busy ratio) into the prometheus registry
//! on each aggregator update tick.
//!
//! `num_workers` / `alive_tasks` / `global_queue_depth` are always available. The per-worker
//! timing (`busy_ratio`, `worker_poll_count`) is only exposed by tokio when the binary is built
//! with `RUSTFLAGS="--cfg tokio_unstable"`; without that flag those two gauges are left at 0.

use crate::node::metrics::handler::{
    MetricsHandler, OnStartMetricsHandler, OnUpdateMetricsHandler,
};
use async_trait::async_trait;
use nym_node_metrics::prometheus_wrapper::{
    NymNodePrometheusMetrics, PROMETHEUS_METRICS, PrometheusMetric,
};
use tokio::runtime::Handle;

// unique marker type so the aggregator can key this handler (it has no real events)
pub struct TokioRuntimeData;

// a snapshot of cumulative worker-busy time, used to derive the busy ratio over the interval
// between two samples
#[cfg(tokio_unstable)]
#[derive(Clone, Copy)]
struct BusySample {
    /// summed busy duration across all workers at the time of the sample
    busy: std::time::Duration,
    /// when the sample was taken
    at: tokio::time::Instant,
}

pub struct TokioRuntimeMetricsUpdater {
    prometheus_wrapper: &'static NymNodePrometheusMetrics,

    // previous busy snapshot, for deriving the busy ratio
    #[cfg(tokio_unstable)]
    prev_busy: Option<BusySample>,
}

impl TokioRuntimeMetricsUpdater {
    pub(crate) fn new() -> Self {
        Self {
            prometheus_wrapper: &PROMETHEUS_METRICS,
            #[cfg(tokio_unstable)]
            prev_busy: None,
        }
    }
}

#[async_trait]
impl OnStartMetricsHandler for TokioRuntimeMetricsUpdater {}

#[async_trait]
impl OnUpdateMetricsHandler for TokioRuntimeMetricsUpdater {
    async fn on_update(&mut self) {
        use PrometheusMetric::*;
        let m = Handle::current().metrics();

        self.prometheus_wrapper
            .set(TokioRuntimeNumWorkers, m.num_workers() as i64);
        self.prometheus_wrapper
            .set(TokioRuntimeAliveTasks, m.num_alive_tasks() as i64);
        self.prometheus_wrapper
            .set(TokioRuntimeGlobalQueueDepth, m.global_queue_depth() as i64);

        // left at their registered 0 unless built with `--cfg tokio_unstable`
        #[cfg(tokio_unstable)]
        {
            let workers = m.num_workers();
            let busy: std::time::Duration =
                (0..workers).map(|w| m.worker_total_busy_duration(w)).sum();
            let now = tokio::time::Instant::now();
            if let Some(prev) = self.prev_busy {
                let elapsed = now.duration_since(prev.at).as_secs_f64();
                let ratio = if workers > 0 && elapsed > 0.0 {
                    busy.saturating_sub(prev.busy).as_secs_f64() / (elapsed * workers as f64)
                } else {
                    0.0
                };
                self.prometheus_wrapper
                    .set_float(TokioRuntimeBusyRatio, ratio);
            }
            self.prev_busy = Some(BusySample { busy, at: now });

            let polls: u64 = (0..workers).map(|w| m.worker_poll_count(w)).sum();
            self.prometheus_wrapper
                .set(TokioRuntimeWorkerPollCount, polls as i64);
        }
    }
}

#[async_trait]
impl MetricsHandler for TokioRuntimeMetricsUpdater {
    type Events = TokioRuntimeData;

    // SAFETY: this handler has no associated events; it only acts on the periodic `on_update`.
    #[allow(clippy::panic)]
    async fn handle_event(&mut self, _event: Self::Events) {
        panic!(
            "MetricsHandler::handle_event incorrectly called on TokioRuntimeMetricsUpdater - it has no events"
        )
    }
}
