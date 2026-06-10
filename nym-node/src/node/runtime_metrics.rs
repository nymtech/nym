// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Periodic tokio runtime-metrics sampler.
//!
//! Surfaces runtime scheduling pressure on the prometheus endpoint as `tokio_runtime_*` gauges so
//! a processing spike can be attributed to runtime starvation (a busy/saturated executor) rather
//! than to any one pipeline stage. The most useful per-worker timing (busy ratio, poll counts) is
//! only exposed by tokio when the binary is built with `RUSTFLAGS="--cfg tokio_unstable"`, so those
//! are gated behind that cfg; the stable queue/task gauges are always emitted.

use nym_task::ShutdownToken;
use std::time::Duration;
use tokio::runtime::Handle;
use tracing::trace;

const SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

pub(crate) async fn run(shutdown_token: ShutdownToken) {
    let handle = Handle::current();
    let registry = nym_metrics::metrics_registry();
    let mut ticker = tokio::time::interval(SAMPLE_INTERVAL);

    #[cfg(tokio_unstable)]
    let mut prev_busy = Duration::ZERO;

    trace!("starting tokio runtime metrics sampler");
    loop {
        tokio::select! {
            _ = shutdown_token.cancelled() => break,
            _ = ticker.tick() => {
                let m = handle.metrics();
                registry.maybe_register_and_set(
                    "tokio_runtime_num_workers",
                    m.num_workers() as i64,
                    "number of tokio worker threads",
                );
                registry.maybe_register_and_set(
                    "tokio_runtime_alive_tasks",
                    m.num_alive_tasks() as i64,
                    "currently alive (spawned, not yet completed) tokio tasks",
                );
                // the headline scheduling-pressure signal: tasks runnable but not yet polled.
                // persistently > 0 means the runtime can't keep up and tasks (e.g. the forwarder)
                // are waiting to be scheduled - exactly the latency a per-stage histogram hides.
                registry.maybe_register_and_set(
                    "tokio_runtime_global_queue_depth",
                    m.global_queue_depth() as i64,
                    "tasks waiting in the tokio global run queue (scheduling pressure)",
                );

                #[cfg(tokio_unstable)]
                {
                    let workers = m.num_workers();
                    let busy: Duration =
                        (0..workers).map(|w| m.worker_total_busy_duration(w)).sum();
                    let delta = busy.saturating_sub(prev_busy);
                    prev_busy = busy;
                    let ratio = if workers > 0 {
                        delta.as_secs_f64() / (SAMPLE_INTERVAL.as_secs_f64() * workers as f64)
                    } else {
                        0.0
                    };
                    registry.maybe_register_and_set_float(
                        "tokio_runtime_busy_ratio",
                        ratio,
                        "fraction of worker-thread time spent busy over the last interval",
                    );
                    let polls: u64 = (0..workers).map(|w| m.worker_poll_count(w)).sum();
                    registry.maybe_register_and_set(
                        "tokio_runtime_worker_poll_count",
                        polls as i64,
                        "cumulative tokio worker poll count across all workers",
                    );
                }
            }
        }
    }
    trace!("tokio runtime metrics sampler: exiting");
}
