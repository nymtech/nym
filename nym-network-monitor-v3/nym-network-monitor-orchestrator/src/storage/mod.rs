// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::storage::manager::StorageManager;
use crate::storage::models::{
    AssignedTestrun, AssignmentRequest, CompletedTestRun, ConfigScoreCandidate, DescribedNodeRoles,
    KindHead, KindSchedule, MixnetEpochAggregate, MixnetEpochConfigScore, NewTestRun,
    NodeAwaitingCapabilityRefresh, NodeChainCapability, NymNode, TestKind, TestRunInProgress,
    TestRunSubmission, TestRunWindow,
};
use anyhow::Context;
use nym_network_monitor_orchestrator_requests::models::{Pagination, RunMeasurements};
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::nym_mixnet_contract_common::EpochId;
use sqlx::ConnectOptions;
use sqlx::sqlite::{SqliteAutoVacuum, SqliteSynchronous};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use strum::IntoEnumIterator;
use time::OffsetDateTime;
use tracing::log::{LevelFilter, debug};

mod manager;
pub(crate) mod models;

/// High-level handle to the orchestrator's local SQLite database.
///
/// Wraps a [`StorageManager`] and translates between the orchestrator-level
/// types (e.g. [`NodeId`], [`Pagination`], [`Duration`]) used by callers and
/// the raw SQL-friendly primitives (`i64` ids, `limit`/`offset`, absolute
/// timestamps) understood by the manager. All public methods are
/// [`Clone`]-safe because [`sqlx::SqlitePool`] is internally reference-counted.
#[derive(Clone)]
pub(crate) struct NetworkMonitorStorage {
    pub(crate) storage_manager: StorageManager,
}

impl NetworkMonitorStorage {
    /// A migrated, empty database held entirely in memory, for tests that need storage without a
    /// file on disk.
    #[cfg(test)]
    pub(crate) async fn in_memory() -> Self {
        let connection_pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("failed to create in-memory SQLite pool");
        let migrations = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        sqlx::migrate::Migrator::new(migrations.as_path())
            .await
            .expect("failed to find migrations")
            .run(&connection_pool)
            .await
            .expect("failed to run migrations");

        NetworkMonitorStorage {
            storage_manager: StorageManager { connection_pool },
        }
    }

    /// Opens (or creates) the SQLite database at `database_path`, configures
    /// WAL journaling and incremental auto-vacuum, and runs the embedded
    /// migrations. Slow statements (>50ms) are logged at `WARN`.
    pub(crate) async fn init<P: AsRef<Path>>(database_path: P) -> anyhow::Result<Self> {
        debug!(
            "attempting to connect to database {}",
            database_path.as_ref().display()
        );

        let connect_opts = sqlx::sqlite::SqliteConnectOptions::new()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .auto_vacuum(SqliteAutoVacuum::Incremental)
            .filename(database_path)
            .create_if_missing(true)
            .log_statements(LevelFilter::Trace)
            .log_slow_statements(LevelFilter::Warn, Duration::from_millis(50));

        let connection_pool = sqlx::SqlitePool::connect_with(connect_opts)
            .await
            .context("Failed to connect to SQLx database")?;

        sqlx::migrate!("./migrations")
            .run(&connection_pool)
            .await
            .context("Failed to run database migrations")?;

        Ok(Self {
            storage_manager: StorageManager { connection_pool },
        })
    }

    /// Records what one refresh learned: every bond it read at `seen_at`, the description of every
    /// node that answered completely, and the removal of the descriptions of nodes no longer bonded.
    pub(crate) async fn store_refresh(
        &self,
        nodes: &[NymNode],
        seen_at: OffsetDateTime,
    ) -> anyhow::Result<()> {
        self.storage_manager.store_refresh(nodes, seen_at).await
    }

    /// Records a submitted result against the run dispatched for its node and releases the node's
    /// lock, in one transaction; see [`StorageManager::submit_testrun`].
    pub(crate) async fn submit_testrun(
        &self,
        run: &NewTestRun,
        measurements: &RunMeasurements,
    ) -> anyhow::Result<TestRunSubmission> {
        self.storage_manager.submit_testrun(run, measurements).await
    }

    /// Releases every in-flight lock whose lease has already expired, on the assumption that those
    /// runs will never report back, counting each kind's expiries against its own child series.
    ///
    /// Takes no timeout: the deadline lives on each row, stamped at dispatch from the budget of the
    /// kind being dispatched, so this sweep needs no knowledge of any kind's lease.
    ///
    /// Returns the total number of locks released.
    pub(crate) async fn clear_expired_testruns_in_progress(&self) -> anyhow::Result<u64> {
        let cleared = self
            .storage_manager
            .clear_expired_testruns_in_progress(OffsetDateTime::now_utc())
            .await?;

        for (kind, count) in &cleared {
            PROMETHEUS_METRICS.inc_by_for_kind(
                PrometheusMetric::TestrunLeasesExpiredTotal,
                *kind,
                *count as i64,
            );
        }

        Ok(cleared.values().sum())
    }

    /// Publishes the per-kind in-flight gauge from the authoritative row counts.
    ///
    /// Set from a count rather than maintained by inc/dec: the delta paths (assign, submit, expire)
    /// would each have to attribute their change to a kind, and a single missed attribution leaves
    /// a child permanently wrong, whereas a recount cannot drift. Every kind is published on every
    /// call, so a kind that has drained reads as zero rather than holding its last value.
    pub(crate) async fn publish_in_progress_gauges(&self) -> anyhow::Result<()> {
        let counts = self
            .storage_manager
            .count_testruns_in_progress_by_kind()
            .await?;

        for kind in TestKind::iter() {
            PROMETHEUS_METRICS.set_for_kind(
                PrometheusMetric::TestrunsInFlight,
                kind,
                counts.get(&kind).copied().unwrap_or_default(),
            );
        }
        Ok(())
    }

    /// Atomically selects the nodes due for one kind and marks each as having a test run in
    /// progress, leased for `schedule.lease_budget` from now. One target for a stress kind, up to
    /// `schedule.wave_size` for a liveness one.
    ///
    /// Resolves the schedule's durations against a single `now`, so every gate applied and every row
    /// stamped by one assignment agrees on when it happened.
    ///
    /// "Most stale" means: nodes this kind has never tested come first, followed by those whose last
    /// run under it is oldest. `staleness_age` is a minimum-staleness gate that never-tested nodes
    /// bypass.
    ///
    /// Nodes with a row in `testrun_in_progress` are excluded whatever kind that row holds, and
    /// become eligible for any kind again as soon as that row clears.
    ///
    /// Returns an empty vector if nothing is eligible.
    pub(crate) async fn assign_next_testruns(
        &self,
        schedule: &KindSchedule,
    ) -> anyhow::Result<Vec<AssignedTestrun>> {
        let now = OffsetDateTime::now_utc();
        let request = AssignmentRequest {
            kind: schedule.kind,
            now,
            last_tested_before: now - schedule.staleness_age,
            expires_at: now + schedule.lease_budget,
            wave_size: schedule.wave_size,
        };

        self.storage_manager.assign_next_testruns(&request).await
    }

    /// When the node `schedule.kind` would assign next fell due, judged against the same staleness
    /// gate the assignment would apply, or `None` if that kind has nothing eligible.
    ///
    /// The node is due at its last test by the kind plus the kind's interval; one the kind has never
    /// tested is due from the start.
    pub(crate) async fn peek_kind_head(
        &self,
        schedule: &KindSchedule,
    ) -> anyhow::Result<Option<KindHead>> {
        let last_tested_before = OffsetDateTime::now_utc() - schedule.staleness_age;
        let head = self
            .storage_manager
            .peek_next_candidate(schedule.kind, last_tested_before)
            .await?;

        Ok(head.map(|candidate| match candidate.last_tested_at {
            None => KindHead::NeverTested,
            Some(last_tested_at) => KindHead::DueAt(last_tested_at + schedule.staleness_age),
        }))
    }

    /// How many nodes `schedule.kind` could be assigned right now, under the same staleness gate
    /// the assignment would apply.
    pub(crate) async fn count_due_candidates(
        &self,
        schedule: &KindSchedule,
    ) -> anyhow::Result<i64> {
        let last_tested_before = OffsetDateTime::now_utc() - schedule.staleness_age;

        match schedule.kind {
            TestKind::MixnodeLiveness => {
                self.storage_manager
                    .count_due_mixnode_liveness_candidates(last_tested_before)
                    .await
            }
            TestKind::GatewayLiveness => {
                self.storage_manager
                    .count_due_gateway_liveness_candidates(last_tested_before)
                    .await
            }
            TestKind::MixnodeStress => {
                self.storage_manager
                    .count_due_mixnode_stress_candidates(last_tested_before)
                    .await
            }
        }
    }

    /// How overdue the oldest node `schedule.kind` has measured at least once is, or `None` when
    /// every node it has measured is still within its cadence.
    ///
    /// Never-measured nodes are excluded, since they have no due time to be overdue against;
    /// [`Self::count_never_tested`] reports them instead.
    pub(crate) async fn head_lateness(
        &self,
        schedule: &KindSchedule,
    ) -> anyhow::Result<Option<Duration>> {
        let now = OffsetDateTime::now_utc();
        let last_tested_before = now - schedule.staleness_age;

        let head = match schedule.kind {
            TestKind::MixnodeLiveness => {
                self.storage_manager
                    .peek_measured_mixnode_liveness_head(last_tested_before)
                    .await?
            }
            TestKind::GatewayLiveness => {
                self.storage_manager
                    .peek_measured_gateway_liveness_head(last_tested_before)
                    .await?
            }
            TestKind::MixnodeStress => {
                self.storage_manager
                    .peek_measured_mixnode_stress_head(last_tested_before)
                    .await?
            }
        };

        Ok(head.map(|last_tested_at| {
            // the head was selected as having fallen due before `now`, so this cannot be negative
            (now - (last_tested_at + schedule.staleness_age)).unsigned_abs()
        }))
    }

    /// How many nodes `kind` could test but holds no result for.
    pub(crate) async fn count_never_tested(&self, kind: TestKind) -> anyhow::Result<i64> {
        match kind {
            TestKind::MixnodeLiveness => {
                self.storage_manager
                    .count_never_tested_mixnode_liveness()
                    .await
            }
            TestKind::GatewayLiveness => {
                self.storage_manager
                    .count_never_tested_gateway_liveness()
                    .await
            }
            TestKind::MixnodeStress => {
                self.storage_manager
                    .count_never_tested_mixnode_stress()
                    .await
            }
        }
    }

    /// How many nodes `kind` can test at all. The population is a property of the role, so the two
    /// mixnode kinds share a count.
    pub(crate) async fn count_testable_population(&self, kind: TestKind) -> anyhow::Result<i64> {
        match kind {
            TestKind::MixnodeLiveness | TestKind::MixnodeStress => {
                self.storage_manager.count_mixnode_role_population().await
            }
            TestKind::GatewayLiveness => self.storage_manager.count_gateway_role_population().await,
        }
    }

    /// Bonded nodes absent from every kind's population, for want of a usable description.
    pub(crate) async fn count_excluded_nodes(&self) -> anyhow::Result<i64> {
        self.storage_manager.count_excluded_nodes().await
    }

    /// The in-flight rows already past their lease, counted by kind. Absent means zero.
    pub(crate) async fn count_expired_testruns_in_progress_by_kind(
        &self,
    ) -> anyhow::Result<HashMap<TestKind, i64>> {
        self.storage_manager
            .count_expired_testruns_in_progress_by_kind(OffsetDateTime::now_utc())
            .await
    }

    /// How long each kind's longest-running dispatch has been outstanding. A kind holding nothing
    /// in flight is absent rather than carrying its last value.
    pub(crate) async fn oldest_in_flight_ages(
        &self,
    ) -> anyhow::Result<HashMap<TestKind, Duration>> {
        let now = OffsetDateTime::now_utc();
        let oldest = self
            .storage_manager
            .oldest_testrun_in_progress_by_kind()
            .await?;

        Ok(oldest
            .into_iter()
            .map(|(kind, started_at)| (kind, (now - started_at).unsigned_abs()))
            .collect())
    }

    /// Fetches one completed run of `test_kind` by its id within that kind, or `None` if it has been
    /// evicted or never existed.
    pub(crate) async fn get_testrun_by_id(
        &self,
        test_kind: TestKind,
        id: i64,
    ) -> anyhow::Result<Option<CompletedTestRun>> {
        self.storage_manager.get_testrun_by_id(test_kind, id).await
    }

    /// Fetches the newest completed run of `test_kind` against a node, or `None` if that kind has
    /// never tested it or its runs have all been evicted.
    pub(crate) async fn get_latest_testrun_for_node(
        &self,
        test_kind: TestKind,
        node_id: NodeId,
    ) -> anyhow::Result<Option<CompletedTestRun>> {
        self.storage_manager
            .get_latest_testrun_for_node(test_kind, node_id as i64)
            .await
    }

    /// Fetches a node by its contract-assigned `node_id`, with its description if it has one, or
    /// `None` if the orchestrator has never observed a bond for it.
    pub(crate) async fn get_nym_node_by_id(
        &self,
        node_id: NodeId,
    ) -> anyhow::Result<Option<NymNode>> {
        self.storage_manager
            .get_nym_node_by_id(node_id as i64)
            .await
    }

    /// Paginated list of outstanding `testrun_in_progress` rows, oldest `started_at`
    /// first so stale/hung runs surface at the top, with the snapshot-consistent
    /// total row count.
    pub(crate) async fn get_testruns_in_progress_paginated(
        &self,
        pagination: Pagination,
    ) -> anyhow::Result<(Vec<TestRunInProgress>, usize)> {
        let (rows, total) = self
            .storage_manager
            .get_testruns_in_progress_paginated(pagination.limit(), pagination.offset())
            .await?;

        Ok((rows, total as usize))
    }

    /// Paginated list of nodes ordered by `node_id` ascending, with the
    /// snapshot-consistent total row count. [`Pagination`] is resolved to
    /// `limit`/`offset` here so the manager never sees the public contract.
    pub(crate) async fn get_nym_nodes_paginated(
        &self,
        pagination: Pagination,
    ) -> anyhow::Result<(Vec<NymNode>, usize)> {
        let (nodes, total) = self
            .storage_manager
            .get_nym_nodes_paginated(pagination.limit(), pagination.offset())
            .await?;

        Ok((nodes, total as usize))
    }

    /// Paginated list of the completed runs of `test_kind`, ordered by `test_timestamp` descending
    /// (newest first), with the snapshot-consistent total row count.
    pub(crate) async fn get_testruns_paginated(
        &self,
        test_kind: TestKind,
        pagination: Pagination,
    ) -> anyhow::Result<(Vec<CompletedTestRun>, usize)> {
        let (test_results, total) = self
            .storage_manager
            .get_testruns_paginated(test_kind, pagination.limit(), pagination.offset())
            .await?;

        Ok((test_results, total as usize))
    }

    /// Paginated list of the completed runs of `test_kind` against a single node, ordered newest
    /// first, with the snapshot-consistent total row count. An unknown or never-tested `node_id`
    /// produces `(vec![], 0)` rather than an error.
    pub(crate) async fn get_testruns_for_node_paginated(
        &self,
        test_kind: TestKind,
        node_id: NodeId,
        pagination: Pagination,
    ) -> anyhow::Result<(Vec<CompletedTestRun>, usize)> {
        let (test_results, total) = self
            .storage_manager
            .get_testruns_for_node_paginated(
                test_kind,
                node_id as i64,
                pagination.limit(),
                pagination.offset(),
            )
            .await?;

        Ok((test_results, total as usize))
    }

    /// Returns the id of the newest run of `test_kind` already submitted to the nym-api, or `None`
    /// if that stream has submitted no batch yet. Callers treat `None` as "submit everything of
    /// that kind currently in storage".
    pub(crate) async fn get_last_submitted_testrun_id(
        &self,
        test_kind: TestKind,
    ) -> anyhow::Result<Option<i64>> {
        self.storage_manager
            .get_last_submitted_testrun_id(test_kind)
            .await
    }

    /// Persists the id of the newest run of `test_kind` whose batch submission to the nym-api has
    /// succeeded. Subsequent [`Self::get_testruns_after`] calls for that kind use this value to
    /// avoid resubmitting already-acknowledged rows.
    pub(crate) async fn set_last_submitted_testrun_id(
        &self,
        test_kind: TestKind,
        testrun_id: i64,
    ) -> anyhow::Result<()> {
        self.storage_manager
            .set_last_submitted_testrun_id(test_kind, testrun_id)
            .await
    }

    /// Fetches every run in `test_kind`'s results table with `id > after_id`, ordered by id
    /// ascending.
    ///
    /// Used by the nym-api submission task to build the next batch of pending results. Ascending
    /// ordering lets the caller record the highest-id row as the new submission watermark once
    /// the batch is acknowledged.
    pub(crate) async fn get_testruns_after(
        &self,
        test_kind: TestKind,
        after_id: i64,
    ) -> anyhow::Result<Vec<CompletedTestRun>> {
        self.storage_manager
            .get_testruns_after(test_kind, after_id)
            .await
    }

    /// Deletes every completed run older than `eviction_age` relative to the current time, from
    /// every kind's results table.
    ///
    /// Intended to be called periodically to keep the local database from growing unboundedly.
    /// Rows that are evicted are assumed to have already been submitted to the nym-api for
    /// persistent storage. Each kind's `last_tested_at` survives, so an evicted result does not make
    /// the node read as never-tested.
    pub(crate) async fn evict_old_testruns(&self, eviction_age: Duration) -> anyhow::Result<u64> {
        let cutoff = OffsetDateTime::now_utc() - eviction_age;
        self.storage_manager.evict_old_testruns(cutoff).await
    }

    /// Every run of `test_kind` stored within `window`, across all nodes.
    pub(crate) async fn get_testruns_in_window(
        &self,
        test_kind: TestKind,
        window: TestRunWindow,
    ) -> anyhow::Result<Vec<CompletedTestRun>> {
        self.storage_manager
            .get_testruns_in_window(test_kind, window)
            .await
    }

    /// Stores aggregates that are not already stored, leaving any that are exactly as they were.
    pub(crate) async fn batch_insert_mixnet_epoch_aggregates(
        &self,
        aggregates: &[MixnetEpochAggregate],
    ) -> anyhow::Result<()> {
        self.storage_manager
            .batch_insert_mixnet_epoch_aggregates(aggregates)
            .await
    }

    /// The newest epoch that has aggregates stored, or `None` when none has.
    pub(crate) async fn get_last_materialised_mixnet_epoch(
        &self,
    ) -> anyhow::Result<Option<EpochId>> {
        let last = self
            .storage_manager
            .get_last_materialised_mixnet_epoch()
            .await?;

        // stored from an `EpochId`, so it fits
        Ok(last.map(|mixnet_epoch| mixnet_epoch as EpochId))
    }

    /// Every aggregate stored for `mixnet_epoch`, ordered by node and then kind.
    pub(crate) async fn get_mixnet_epoch_aggregates(
        &self,
        mixnet_epoch: EpochId,
    ) -> anyhow::Result<Vec<MixnetEpochAggregate>> {
        self.storage_manager
            .get_mixnet_epoch_aggregates(i64::from(mixnet_epoch))
            .await
    }

    /// One node's aggregates for `mixnet_epoch`, one per kind that measured it.
    pub(crate) async fn get_mixnet_epoch_aggregates_for_node(
        &self,
        mixnet_epoch: EpochId,
        node_id: NodeId,
    ) -> anyhow::Result<Vec<MixnetEpochAggregate>> {
        self.storage_manager
            .get_mixnet_epoch_aggregates_for_node(i64::from(mixnet_epoch), node_id as i64)
            .await
    }

    /// Deletes every aggregate of an epoch that began longer than `retention` ago.
    pub(crate) async fn evict_old_mixnet_epoch_aggregates(
        &self,
        retention: Duration,
    ) -> anyhow::Result<u64> {
        let cutoff = OffsetDateTime::now_utc() - retention;
        self.storage_manager
            .evict_old_mixnet_epoch_aggregates(cutoff)
            .await
    }

    /// Stores the on-chain standing of every node in `capabilities`, replacing what was cached.
    pub(crate) async fn batch_upsert_node_chain_capabilities(
        &self,
        capabilities: &[NodeChainCapability],
    ) -> anyhow::Result<()> {
        self.storage_manager
            .batch_upsert_node_chain_capabilities(capabilities)
            .await
    }

    /// Every described node whose on-chain standing is not cached yet or has fallen due.
    pub(crate) async fn get_nodes_awaiting_capability_refresh(
        &self,
    ) -> anyhow::Result<Vec<NodeAwaitingCapabilityRefresh>> {
        self.storage_manager
            .get_nodes_awaiting_capability_refresh(OffsetDateTime::now_utc())
            .await
    }

    /// Every described node with its cached on-chain standing, if any.
    pub(crate) async fn get_config_score_candidates(
        &self,
    ) -> anyhow::Result<Vec<ConfigScoreCandidate>> {
        self.storage_manager.get_config_score_candidates().await
    }

    /// The declared roles of every described node, ordered by node id.
    pub(crate) async fn get_described_node_roles(&self) -> anyhow::Result<Vec<DescribedNodeRoles>> {
        self.storage_manager.get_described_node_roles().await
    }

    /// Stores config scores that are not already stored, leaving any that are exactly as they were.
    pub(crate) async fn batch_insert_mixnet_epoch_config_scores(
        &self,
        scores: &[MixnetEpochConfigScore],
    ) -> anyhow::Result<()> {
        self.storage_manager
            .batch_insert_mixnet_epoch_config_scores(scores)
            .await
    }

    /// Every config score stored for `mixnet_epoch`, ordered by node.
    pub(crate) async fn get_mixnet_epoch_config_scores(
        &self,
        mixnet_epoch: EpochId,
    ) -> anyhow::Result<Vec<MixnetEpochConfigScore>> {
        self.storage_manager
            .get_mixnet_epoch_config_scores(i64::from(mixnet_epoch))
            .await
    }

    /// One node's config score for `mixnet_epoch`, or `None` if none was stored.
    pub(crate) async fn get_mixnet_epoch_config_score_for_node(
        &self,
        mixnet_epoch: EpochId,
        node_id: NodeId,
    ) -> anyhow::Result<Option<MixnetEpochConfigScore>> {
        self.storage_manager
            .get_mixnet_epoch_config_score_for_node(i64::from(mixnet_epoch), node_id as i64)
            .await
    }

    /// Deletes every config score of an epoch that began longer than `retention` ago.
    pub(crate) async fn evict_old_mixnet_epoch_config_scores(
        &self,
        retention: Duration,
    ) -> anyhow::Result<u64> {
        let cutoff = OffsetDateTime::now_utc() - retention;
        self.storage_manager
            .evict_old_mixnet_epoch_config_scores(cutoff)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::{
        FIXTURE_SEEN_AT, bond_only, described_node, gateway, minimal_measurements,
        minimal_test_run, mixnode,
    };

    /// The shipped liveness cadence, which is what the overdue figures below are judged against.
    const CADENCE: Duration = Duration::from_secs(15 * 60);

    async fn storage_with(nodes: &[NymNode]) -> NetworkMonitorStorage {
        let storage = NetworkMonitorStorage::in_memory().await;
        storage.store_refresh(nodes, FIXTURE_SEEN_AT).await.unwrap();
        storage
    }

    fn schedule(kind: TestKind, wave_size: usize) -> KindSchedule {
        KindSchedule {
            kind,
            staleness_age: CADENCE,
            lease_budget: Duration::from_secs(60),
            wave_size,
        }
    }

    /// Records a `kind` run against `node_id` finished `ago` before now, which is what sets the
    /// node's due time for that kind.
    async fn tested(storage: &NetworkMonitorStorage, kind: TestKind, node_id: i64, ago: Duration) {
        let run = NewTestRun {
            test_timestamp: OffsetDateTime::now_utc() - ago,
            ..minimal_test_run(node_id)
        };
        storage
            .storage_manager
            .insert_test_run(&run, &minimal_measurements(kind))
            .await
            .unwrap();
    }

    /// Locks `node_id` under `kind`, dispatched `started_ago` back and leased for `lease` from then,
    /// so a lease shorter than the dispatch age is one the sweep has not yet reaped.
    async fn in_flight(
        storage: &NetworkMonitorStorage,
        kind: TestKind,
        node_id: i64,
        started_ago: Duration,
        lease: Duration,
    ) {
        let started_at = OffsetDateTime::now_utc() - started_ago;
        storage
            .storage_manager
            .mark_testrun_in_progress(node_id, started_at, started_at + lease, kind)
            .await
            .unwrap()
    }

    async fn in_flight_counts(storage: &NetworkMonitorStorage) -> HashMap<TestKind, i64> {
        storage
            .storage_manager
            .count_testruns_in_progress_by_kind()
            .await
            .unwrap()
    }

    fn minutes(minutes: u64) -> Duration {
        Duration::from_secs(minutes * 60)
    }

    // Both figures come out of one predicate, so the count cannot report a population the
    // assignment would not have drawn from. Asserted by taking both against one fixture.
    #[tokio::test]
    async fn the_due_count_is_drawn_from_the_set_an_assignment_locks() {
        let storage = storage_with(&[mixnode(1), mixnode(2), gateway(3)]).await;
        let schedule = schedule(TestKind::MixnodeLiveness, 10);

        let due = storage.count_due_candidates(&schedule).await.unwrap();
        let assigned = storage.assign_next_testruns(&schedule).await.unwrap();

        assert_eq!(2, due, "the gateway is not a mixnode liveness candidate");
        assert_eq!(due as usize, assigned.len());
    }

    // The case that would otherwise read as spare capacity: nothing is assignable, but only because
    // everything due is already locked.
    #[tokio::test]
    async fn a_saturated_kind_reports_no_due_work_while_holding_runs_in_flight() {
        let storage = storage_with(&[mixnode(1), mixnode(2)]).await;
        let schedule = schedule(TestKind::MixnodeLiveness, 10);
        in_flight(
            &storage,
            TestKind::MixnodeLiveness,
            1,
            Duration::ZERO,
            minutes(1),
        )
        .await;
        in_flight(
            &storage,
            TestKind::MixnodeLiveness,
            2,
            Duration::ZERO,
            minutes(1),
        )
        .await;

        assert_eq!(0, storage.count_due_candidates(&schedule).await.unwrap());
        assert_eq!(
            Some(&2),
            in_flight_counts(&storage)
                .await
                .get(&TestKind::MixnodeLiveness)
        );
    }

    // An expired lease still holds its node, so it is counted in both places rather than deducted
    // from one: in flight says what is locked, expired says how much of that is abandoned.
    #[tokio::test]
    async fn a_dispatch_past_its_lease_is_counted_both_as_in_flight_and_as_expired() {
        let storage = storage_with(&[mixnode(1)]).await;
        in_flight(
            &storage,
            TestKind::MixnodeLiveness,
            1,
            minutes(5),
            minutes(1),
        )
        .await;

        let expired = storage
            .count_expired_testruns_in_progress_by_kind()
            .await
            .unwrap();

        assert_eq!(
            Some(&1),
            in_flight_counts(&storage)
                .await
                .get(&TestKind::MixnodeLiveness)
        );
        assert_eq!(Some(&1), expired.get(&TestKind::MixnodeLiveness));
    }

    // A node the fleet cannot test must not flatter utilisation by being absent from the
    // denominator without being visible anywhere else.
    #[tokio::test]
    async fn a_bonded_node_without_a_usable_description_is_excluded_and_counted() {
        let storage = storage_with(&[
            mixnode(1),
            bond_only(mixnode(2)),
            described_node(3, "1.2.3.4", false, false),
        ])
        .await;

        for kind in TestKind::iter() {
            let population = storage.count_testable_population(kind).await.unwrap();
            let expected = match kind {
                TestKind::MixnodeLiveness | TestKind::MixnodeStress => 1,
                TestKind::GatewayLiveness => 0,
            };
            assert_eq!(expected, population, "{kind} population");
        }

        assert_eq!(2, storage.count_excluded_nodes().await.unwrap());
    }

    // The leading indicator: a wave closing on its deadline is visible before anything breaches.
    #[tokio::test]
    async fn an_outstanding_dispatch_ages_while_its_expired_count_stays_zero() {
        let storage = storage_with(&[mixnode(1)]).await;
        in_flight(
            &storage,
            TestKind::MixnodeLiveness,
            1,
            minutes(10),
            minutes(30),
        )
        .await;

        let ages = storage.oldest_in_flight_ages().await.unwrap();
        let expired = storage
            .count_expired_testruns_in_progress_by_kind()
            .await
            .unwrap();

        assert!(ages[&TestKind::MixnodeLiveness] >= minutes(10));
        assert_eq!(None, expired.get(&TestKind::MixnodeLiveness));
        // a kind holding nothing is absent rather than carrying a stale age
        assert_eq!(None, ages.get(&TestKind::GatewayLiveness));
    }

    // The case that makes head lateness unusable if the two are folded together: the assignment
    // order puts a never-tested node at the head, and a live network always has one.
    #[tokio::test]
    async fn a_never_tested_node_is_counted_without_disturbing_head_lateness() {
        let storage = storage_with(&[mixnode(1)]).await;
        let schedule = schedule(TestKind::MixnodeLiveness, 10);
        tested(&storage, TestKind::MixnodeLiveness, 1, minutes(20)).await;

        let before = storage.head_lateness(&schedule).await.unwrap();

        // a newly bonded node arrives that the kind has never measured
        storage
            .store_refresh(&[mixnode(1), mixnode(2)], FIXTURE_SEEN_AT)
            .await
            .unwrap();
        let after = storage.head_lateness(&schedule).await.unwrap();

        assert_eq!(
            1,
            storage
                .count_never_tested(TestKind::MixnodeLiveness)
                .await
                .unwrap()
        );
        // 20 minutes measured against a 15 minute cadence, before and after
        for lateness in [before, after] {
            let lateness = lateness.expect("the measured head is overdue");
            assert!(
                lateness >= minutes(4) && lateness <= minutes(6),
                "{lateness:?}"
            );
        }
    }
}
