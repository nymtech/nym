// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::storage::NetworkMonitorStorage;
use nym_task::ShutdownToken;
use std::time::Duration;
use tokio::time::{Instant, MissedTickBehavior, interval_at};
use tracing::{debug, error, info};

/// The value configuration the eviction task needs, grouped so construction takes one bundle
/// rather than a long argument list.
pub(crate) struct EvictionConfig {
    /// Age past which a finalised test run is considered stale and removed.
    pub(crate) testrun_eviction_age: Duration,

    /// Age past which an epoch aggregate or config score is removed.
    pub(crate) aggregate_retention: Duration,

    /// Cadence at which expired leases are released.
    pub(crate) lease_release_interval: Duration,

    /// Cadence at which rows past their retention are deleted.
    pub(crate) retention_sweep_interval: Duration,
}

/// Background task that periodically purges stale data from the storage.
///
/// Three distinct kinds of staleness are handled:
/// - in-progress test runs whose lease has expired (freed so they can be
///   reassigned),
/// - finalised test runs older than `testrun_eviction_age` (dropped to keep
///   the results table bounded),
/// - epoch aggregates and config scores older than `aggregate_retention`
///   (dropped to keep the per-epoch tables bounded).
///
/// The first of those is on its own timer, because the two duties want opposite cadences. A lease
/// still held past its deadline excludes its node from every assignment, so releasing it wants to
/// happen well inside the shortest lease; deleting rows past a multi-day retention is expensive and
/// has no business running every half minute. One timer sized for either is wrong for the other,
/// which is what the single derived interval it replaces got wrong: computed from the stress lease
/// alone, it left an abandoned liveness wave holding its targets for several times its own lease.
///
/// The deletions are deliberately issued as separate statements rather than
/// wrapped in a transaction: they touch disjoint tables, a partial failure is
/// self-healing on the next tick, and keeping them independent avoids holding
/// a write lock across all of them for the whole sweep.
pub(crate) struct StaleResultsEviction {
    storage: NetworkMonitorStorage,
    config: EvictionConfig,
    shutdown_token: ShutdownToken,
}

impl StaleResultsEviction {
    pub(crate) fn new(
        config: EvictionConfig,
        storage: NetworkMonitorStorage,
        shutdown_token: ShutdownToken,
    ) -> Self {
        Self {
            storage,
            config,
            shutdown_token,
        }
    }

    /// Releases every in-flight lock whose lease has already expired, freeing its node for
    /// reassignment, and returns how many were released.
    ///
    /// Each row is compared against the deadline stamped on it at dispatch, so this needs no
    /// knowledge of any kind's lease and a kind added later needs no change here.
    pub(crate) async fn release_expired_leases(&self) -> anyhow::Result<u64> {
        let released = self.storage.clear_expired_testruns_in_progress().await?;
        if released > 0 {
            info!(released, "released expired testrun leases");
        } else {
            debug!("no expired testrun leases to release");
        }

        Ok(released)
    }

    /// Deletes finalised runs past `testrun_eviction_age`, and aggregates and config scores past
    /// `aggregate_retention`. Logs how many rows were affected so ops can confirm the task is doing
    /// real work (and spot unexpected spikes).
    async fn prune_past_retention(&self) -> anyhow::Result<()> {
        let evicted_old = self
            .storage
            .evict_old_testruns(self.config.testrun_eviction_age)
            .await?;
        let evicted_aggregates = self
            .storage
            .evict_old_mixnet_epoch_aggregates(self.config.aggregate_retention)
            .await?;
        let evicted_config_scores = self
            .storage
            .evict_old_mixnet_epoch_config_scores(self.config.aggregate_retention)
            .await?;

        if evicted_old > 0 || evicted_aggregates > 0 || evicted_config_scores > 0 {
            PROMETHEUS_METRICS.inc_by(PrometheusMetric::StaleTestrunsEvicted, evicted_old as i64);

            info!(
                evicted_old,
                evicted_aggregates, evicted_config_scores, "retention sweep completed"
            );
        } else {
            debug!("retention sweep completed: nothing to evict");
        }

        Ok(())
    }

    /// Runs both passes on their own timers until the shutdown token is cancelled.
    ///
    /// Cancellation is cooperative: it is only observed between passes, so a
    /// pass already in flight is allowed to finish. This keeps partial
    /// deletions from being left on the floor at shutdown.
    ///
    /// Both first ticks are deliberately offset by their own interval. The orchestrator releases
    /// expired leases once during start-up (to reap anything left behind by a prior crash or
    /// restart), so an immediate tick would redo that work, and a retention sweep has nothing to do
    /// at boot that it will not still have to do an interval later.
    ///
    /// `MissedTickBehavior::Delay` prevents burst catch-up ticks when a pass
    /// runs long under DB load — otherwise a slow pass would queue multiple
    /// back-to-back ticks and amplify the pressure that made it slow in the
    /// first place.
    pub(crate) async fn run(&self) {
        let mut lease_release = offset_interval(self.config.lease_release_interval);
        let mut retention_sweep = offset_interval(self.config.retention_sweep_interval);

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => break,
                _ = lease_release.tick() => {
                    if let Err(err) = self.release_expired_leases().await {
                        // Transient storage errors shouldn't kill the task — the next
                        // tick will retry and any missed items simply age a bit longer.
                        error!("failed to release expired testrun leases: {err}");
                    }
                }
                _ = retention_sweep.tick() => {
                    if let Err(err) = self.prune_past_retention().await {
                        error!("failed to prune data past retention: {err}");
                    }
                }
            }
        }

        info!("stale results eviction stopped");
    }
}

/// An interval whose first tick is one whole period away rather than immediate.
fn offset_interval(period: Duration) -> tokio::time::Interval {
    let mut interval = interval_at(Instant::now() + period, period);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    interval
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::{FIXTURE_SEEN_AT, TestKind, mixnode};
    use time::OffsetDateTime;

    fn eviction(storage: NetworkMonitorStorage) -> StaleResultsEviction {
        StaleResultsEviction::new(
            EvictionConfig {
                testrun_eviction_age: Duration::from_secs(7 * 24 * 60 * 60),
                aggregate_retention: Duration::from_secs(30 * 24 * 60 * 60),
                lease_release_interval: Duration::from_secs(30),
                retention_sweep_interval: Duration::from_secs(60 * 60),
            },
            storage,
            ShutdownToken::new(),
        )
    }

    // The point of the split: an abandoned liveness wave used to sit on its targets until a sweep
    // sized from the stress lease came round, several times its own 1 minute lease later.
    #[tokio::test]
    async fn an_expired_liveness_lease_is_released_without_a_retention_sweep() {
        let storage = NetworkMonitorStorage::in_memory().await;
        storage
            .store_refresh(&[mixnode(1)], FIXTURE_SEEN_AT)
            .await
            .unwrap();

        // dispatched 5 minutes ago under a 1 minute lease, so it is long overdue
        let started_at = OffsetDateTime::now_utc() - Duration::from_secs(5 * 60);
        storage
            .storage_manager
            .mark_testrun_in_progress(
                1,
                started_at,
                started_at + Duration::from_secs(60),
                TestKind::MixnodeLiveness,
            )
            .await
            .unwrap();

        let released = eviction(storage.clone())
            .release_expired_leases()
            .await
            .unwrap();

        assert_eq!(1, released);
        assert!(
            storage
                .count_testruns_in_progress_by_kind()
                .await
                .unwrap()
                .is_empty()
        );
    }
}
