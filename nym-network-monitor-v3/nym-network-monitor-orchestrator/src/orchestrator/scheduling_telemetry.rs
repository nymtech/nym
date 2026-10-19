// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The scheduling-telemetry task: the per-kind capacity picture, recomputed from storage on its own
//! tick.
//!
//! The orchestrator hands out work only when an agent asks for it, so achieved throughput says
//! nothing about available capacity: the fleet does as much work as there is work to do and no
//! more. What does say something is the queue itself, and the gap between a kind's work draining
//! and becoming due again is its spare capacity. Everything here is read from the scheduler's own
//! tables, with the due count taken over the assignment's own predicate so the two can never
//! disagree.
//!
//! The task owns its tick rather than extending the eviction sweep, whose cadence is derived from a
//! lease measured in minutes and cannot resolve one measured in seconds.

use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::{KindSchedule, TestKind};
use nym_task::ShutdownToken;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use strum::IntoEnumIterator;
use tokio::time::MissedTickBehavior;
use tracing::{info, warn};

/// The value configuration the telemetry pass needs, grouped so construction takes one bundle
/// rather than a long argument list.
pub(crate) struct TelemetryConfig {
    /// How often the scheduling picture is recomputed. Should sit at or below the Prometheus scrape
    /// interval, so no sampled series is stale by the time it is read.
    pub(crate) refresh_interval: Duration,

    /// The schedule of every kind that is switched on. A kind that is off still has its population
    /// and never-tested count published, since those nodes exist and are unmeasured, but nothing
    /// derived from a cadence it does not have.
    pub(crate) schedules: Vec<KindSchedule>,
}

/// Publishes the per-kind scheduling picture: what is due, what is in flight, how overdue the queue
/// head is, and how much of the time each kind has work at all.
pub(crate) struct SchedulingTelemetry {
    storage: NetworkMonitorStorage,
    config: TelemetryConfig,
    shutdown_token: ShutdownToken,

    /// Pending time not yet carried into the counter, per kind.
    ///
    /// The counter is whole seconds, while a tick is whatever the configured interval and the
    /// runtime between them produce, so the sub-second part of every tick is carried here instead
    /// of being truncated away. Over an hour at a 15 second tick that is the difference between
    /// reporting utilisation and reporting about nine tenths of it.
    pending_remainders: HashMap<TestKind, Duration>,
}

impl SchedulingTelemetry {
    pub(crate) fn new(
        config: TelemetryConfig,
        storage: NetworkMonitorStorage,
        shutdown_token: ShutdownToken,
    ) -> Self {
        SchedulingTelemetry {
            storage,
            config,
            shutdown_token,
            pending_remainders: HashMap::new(),
        }
    }

    /// One pass: read the scheduling picture for every kind and publish it.
    ///
    /// `elapsed` is the measured time since the previous pass, which is what the pending-time
    /// counter advances by. Measured rather than assumed to be the configured interval, so a
    /// retuned interval or a tick the runtime delayed stays correct instead of quietly rescaling
    /// every utilisation figure.
    ///
    /// Reads only, and SQLite runs in WAL mode, so none of this blocks the assignment path's write
    /// transactions.
    async fn refresh(&mut self, elapsed: Duration) -> anyhow::Result<()> {
        // the three reads that cover every kind at once
        let in_flight = self.storage.count_testruns_in_progress_by_kind().await?;
        let expired = self
            .storage
            .count_expired_testruns_in_progress_by_kind()
            .await?;
        let oldest_ages = self.storage.oldest_in_flight_ages().await?;

        PROMETHEUS_METRICS.set(
            PrometheusMetric::ExcludedNodes,
            self.storage.count_excluded_nodes().await?,
        );

        for kind in TestKind::iter() {
            let in_flight = in_flight.get(&kind).copied().unwrap_or_default();
            let expired = expired.get(&kind).copied().unwrap_or_default();

            // a kind holding nothing in flight reports zero rather than its last value, so an idle
            // kind is never mistaken for one sitting on a very old dispatch
            let oldest_age = oldest_ages.get(&kind).copied().unwrap_or_default();

            PROMETHEUS_METRICS.set_for_kind(PrometheusMetric::TestrunsInFlight, kind, in_flight);
            PROMETHEUS_METRICS.set_for_kind(PrometheusMetric::InFlightExpired, kind, expired);
            PROMETHEUS_METRICS.set_for_kind(
                PrometheusMetric::OldestInFlightAgeSeconds,
                kind,
                oldest_age.as_secs() as i64,
            );

            let population = self.storage.count_testable_population(kind).await?;
            PROMETHEUS_METRICS.set_for_kind(PrometheusMetric::TestablePopulation, kind, population);
            PROMETHEUS_METRICS.set_for_kind(
                PrometheusMetric::NeverTested,
                kind,
                self.storage.count_never_tested(kind).await?,
            );

            let Some(schedule) = self.schedule(kind) else {
                continue;
            };

            let pending = self.storage.count_due_candidates(schedule).await?;
            let head_lateness = self.storage.head_lateness(schedule).await?;

            PROMETHEUS_METRICS.set_for_kind(PrometheusMetric::TestrunsPending, kind, pending);
            PROMETHEUS_METRICS.set_for_kind(
                PrometheusMetric::HeadLatenessSeconds,
                kind,
                head_lateness.unwrap_or_default().as_secs() as i64,
            );
            PROMETHEUS_METRICS.set_float_for_kind(
                PrometheusMetric::RequiredAssignmentRate,
                kind,
                required_assignment_rate(population, schedule),
            );

            // pending and in-flight together: both zero is genuine idle, pending zero with work in
            // flight is saturation, and only the first of those is spare capacity
            if pending > 0 || in_flight > 0 {
                self.accumulate_pending_time(kind, elapsed);
            }
        }

        PROMETHEUS_METRICS.inc(PrometheusMetric::MetricsTaskTicksTotal);
        Ok(())
    }

    /// How `kind` is scheduled, or `None` while it is switched off.
    fn schedule(&self, kind: TestKind) -> Option<&KindSchedule> {
        self.config
            .schedules
            .iter()
            .find(|schedule| schedule.kind == kind)
    }

    /// Advances `kind`'s pending-time counter by the whole seconds of `elapsed`, keeping what is
    /// left over for the next tick.
    fn accumulate_pending_time(&mut self, kind: TestKind, elapsed: Duration) {
        let carried = self.pending_remainders.entry(kind).or_default();
        *carried += elapsed;

        let whole_seconds = carried.as_secs();
        if whole_seconds > 0 {
            *carried -= Duration::from_secs(whole_seconds);
            PROMETHEUS_METRICS.inc_by_for_kind(
                PrometheusMetric::WorkPendingSecondsTotal,
                kind,
                whole_seconds as i64,
            );
        }
    }

    /// Publishes on startup and then every `refresh_interval`. A failed pass is logged and left for
    /// the next one rather than killing the task, which would freeze every series it owns while the
    /// orchestrator carried on serving.
    pub(crate) async fn run(mut self) {
        let mut interval = tokio::time::interval(self.config.refresh_interval);
        // catching up on missed ticks would publish the same picture several times over while
        // crediting each pass almost no elapsed time, so a late tick simply runs late
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut last_tick = Instant::now();

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => break,
                _ = interval.tick() => {
                    let elapsed = last_tick.elapsed();
                    last_tick = Instant::now();

                    if let Err(err) = self.refresh(elapsed).await {
                        warn!("scheduling telemetry refresh failed: {err}");
                    }
                }
            }
        }

        info!("scheduling telemetry stopped");
    }
}

/// The assignments per second `kind` must sustain to hold its cadence: its testable population over
/// its staleness interval, divided by how many targets one assignment carries.
///
/// The configured wave size rather than the observed one, which collapses towards a handful of
/// targets exactly when a kind's queue has drained and capacity is most plentiful, and so would
/// overstate the requirement precisely when there is the most slack.
fn required_assignment_rate(population: i64, schedule: &KindSchedule) -> f64 {
    population as f64 / schedule.staleness_age.as_secs_f64() / schedule.wave_size as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::config::test_config;
    use crate::storage::models::{FIXTURE_SEEN_AT, mixnode};

    fn telemetry(storage: NetworkMonitorStorage) -> SchedulingTelemetry {
        SchedulingTelemetry::new(
            TelemetryConfig {
                refresh_interval: Duration::from_secs(15),
                schedules: test_config(true).schedules(),
            },
            storage,
            ShutdownToken::new(),
        )
    }

    /// The counter's current value for `kind`, read back out of the exposition the scrape endpoint
    /// renders. The registry is process-wide and has no reset, so tests assert on deltas.
    fn pending_seconds(kind: TestKind) -> u64 {
        let series = format!(
            "nym_network_monitor_work_pending_seconds_total{{kind=\"{}\"}} ",
            <&'static str>::from(kind)
        );

        PROMETHEUS_METRICS
            .metrics()
            .lines()
            .find_map(|line| line.strip_prefix(&series))
            .expect("the counter is pre-created by initialise")
            .parse()
            .expect("a counter renders as a number")
    }

    // The idle gap is the whole measurement: a kind with nothing due and nothing in flight must
    // leave the counter alone, or every utilisation figure reads 100% and headroom vanishes.
    #[tokio::test]
    async fn the_pending_time_counter_advances_only_while_a_kind_has_work() {
        let storage = NetworkMonitorStorage::in_memory().await;
        let mut telemetry = telemetry(storage.clone());
        let before = pending_seconds(TestKind::MixnodeLiveness);

        // nothing is bonded, so every kind is idle
        telemetry.refresh(Duration::from_secs(5)).await.unwrap();
        assert_eq!(before, pending_seconds(TestKind::MixnodeLiveness));

        // one never-tested node is work mixnode liveness could be assigned right now
        storage
            .store_refresh(&[mixnode(1)], FIXTURE_SEEN_AT)
            .await
            .unwrap();
        telemetry.refresh(Duration::from_secs(5)).await.unwrap();
        assert_eq!(before + 5, pending_seconds(TestKind::MixnodeLiveness));
    }

    // A tick is whatever the runtime delivers rather than a whole number of seconds, so the
    // sub-second part is carried instead of truncated: three 500ms ticks owe one second, not zero.
    #[tokio::test]
    async fn sub_second_ticks_are_carried_rather_than_truncated() {
        let storage = NetworkMonitorStorage::in_memory().await;
        let mut telemetry = telemetry(storage);

        for _ in 0..3 {
            telemetry.accumulate_pending_time(TestKind::MixnodeStress, Duration::from_millis(500));
        }

        assert_eq!(
            Duration::from_millis(500),
            telemetry.pending_remainders[&TestKind::MixnodeStress]
        );
    }
}
