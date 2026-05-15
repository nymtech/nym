// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::storage::NetworkMonitorStorage;
use nym_task::ShutdownToken;
use std::time::Duration;
use tokio::time::{Instant, MissedTickBehavior, interval_at};
use tracing::{debug, error, info};

/// Background task that periodically purges stale data from the storage.
///
/// Two distinct kinds of staleness are handled:
/// - in-progress test runs whose assigned agent has gone silent past
///   `test_timeout` (freed so they can be reassigned),
/// - finalised test runs older than `testrun_eviction_age` (dropped to keep
///   the results table bounded).
///
/// The two deletions are deliberately issued as separate statements rather
/// than wrapped in a transaction: they touch disjoint tables, a partial
/// failure is self-healing on the next tick, and keeping them independent
/// avoids holding a write lock across both for the whole sweep.
pub(crate) struct StaleResultsEviction {
    storage: NetworkMonitorStorage,

    /// Age past which a finalised test run is considered stale and removed.
    /// Mirrors `Config::testrun_eviction_age`.
    testrun_eviction_age: Duration,

    /// Maximum time a test run may remain "in progress" before we assume the
    /// assigned agent has died and free the slot for reassignment.
    /// Mirrors `Config::test_timeout`.
    test_timeout: Duration,

    /// Cadence at which [`Self::run`] performs an eviction sweep.
    check_interval: Duration,

    shutdown_token: ShutdownToken,
}

/// Lower bound on the sweep cadence to avoid hammering the DB (or panicking
/// `interval_at`) when either timeout is configured to an unrealistically
/// small value.
const MIN_CHECK_INTERVAL: Duration = Duration::from_secs(60);

impl StaleResultsEviction {
    pub(crate) fn new(
        storage: NetworkMonitorStorage,
        testrun_eviction_age: Duration,
        test_timeout: Duration,
        shutdown_token: ShutdownToken,
    ) -> Self {
        // Sweep at least twice per shortest timeout window so the worst-case
        // lag between an item going stale and being evicted is bounded by
        // roughly 1.5x that timeout rather than 2x. Floored at
        // `MIN_CHECK_INTERVAL` to stay safe under degenerate configs.
        let check_interval = Duration::max(
            MIN_CHECK_INTERVAL,
            Duration::min(testrun_eviction_age, test_timeout) / 2,
        );

        Self {
            storage,
            testrun_eviction_age,
            test_timeout,
            check_interval,
            shutdown_token,
        }
    }

    /// Performs a single eviction sweep: clears timed-out in-progress test
    /// runs and deletes results older than the configured retention window.
    /// Logs how many rows were affected so ops can confirm the task is doing
    /// real work (and spot unexpected spikes).
    pub(crate) async fn evict_stale_results(&self) -> anyhow::Result<()> {
        let cleared_in_progress = self
            .storage
            .clear_timed_out_testruns_in_progress(self.test_timeout)
            .await?;
        let evicted_old = self
            .storage
            .evict_old_testruns(self.testrun_eviction_age)
            .await?;

        if cleared_in_progress > 0 || evicted_old > 0 {
            PROMETHEUS_METRICS.inc_by(
                PrometheusMetric::TimedOutTestrunsEvicted,
                cleared_in_progress as i64,
            );
            PROMETHEUS_METRICS.inc_by(PrometheusMetric::StaleTestrunsEvicted, evicted_old as i64);

            info!(
                cleared_in_progress,
                evicted_old, "stale data eviction sweep completed"
            );
        } else {
            debug!("stale data eviction sweep completed: nothing to evict");
        }

        // Reconcile the in-flight gauge against the authoritative row count. The gauge is
        // primarily maintained live via inc/dec at assign/submit/timeout paths; this sweep is
        // a safety net that corrects any drift (e.g. from a future code path that forgets to
        // update the gauge) and bounds the worst-case staleness to one sweep interval.
        match self.storage.count_testruns_in_progress().await {
            Ok(count) => PROMETHEUS_METRICS.set(PrometheusMetric::TestrunsInProgress, count),
            Err(err) => error!("failed to count in-flight testruns for metric: {err}"),
        }
        Ok(())
    }

    /// Runs the eviction loop until the shutdown token is cancelled.
    ///
    /// Cancellation is cooperative: it is only observed between sweeps, so a
    /// sweep already in flight is allowed to finish. This keeps partial
    /// deletions from being left on the floor at shutdown.
    ///
    /// The first tick is deliberately offset by `check_interval` because the
    /// orchestrator invokes [`Self::evict_stale_results`] once during start-up
    /// (to reap anything left behind by a prior crash or restart), so an
    /// immediate tick here would redo that work.
    ///
    /// `MissedTickBehavior::Delay` prevents burst catch-up ticks when a sweep
    /// runs long under DB load — otherwise a slow sweep would queue multiple
    /// back-to-back ticks and amplify the pressure that made it slow in the
    /// first place.
    pub(crate) async fn run(&self) {
        let mut interval = interval_at(Instant::now() + self.check_interval, self.check_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => break,
                _ = interval.tick() => {
                    if let Err(err) = self.evict_stale_results().await {
                        // Transient storage errors shouldn't kill the task — the next
                        // tick will retry and any missed items simply age a bit longer.
                        error!("failed to evict stale results: {err}");
                    }
                }
            }
        }

        info!("stale results eviction stopped");
    }
}
