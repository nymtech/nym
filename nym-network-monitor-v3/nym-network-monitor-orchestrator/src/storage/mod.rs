// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use crate::storage::manager::StorageManager;
use crate::storage::models::{
    AssignedTestrun, CompletedTestRun, NewNymNode, NewTestRun, NymNode, TestKind,
    TestRunInProgress, TestRunMeasurement,
};
use anyhow::Context;
use nym_network_monitor_orchestrator_requests::models::Pagination;
use nym_validator_client::client::NodeId;
use sqlx::ConnectOptions;
use sqlx::sqlite::{SqliteAutoVacuum, SqliteSynchronous};
use std::path::Path;
use std::time::Duration;
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

    /// Inserts or updates multiple node records in a single transaction.
    ///
    /// For each node, if a row with the same `node_id` already exists, all fields except
    /// `identity_key` are updated. The entire batch shares one transaction for efficiency.
    pub(crate) async fn batch_insert_or_update_nym_nodes(
        &self,
        nodes: &[NewNymNode],
    ) -> anyhow::Result<()> {
        self.storage_manager
            .batch_insert_or_update_nym_nodes(nodes)
            .await
    }

    /// Persists a completed test run with its measurements, records the work state of the
    /// (kind, role) pairing it belongs to, and releases the node's in-flight lock — all in one
    /// transaction.
    ///
    /// Decrements the `TestrunsInProgress` gauge iff a lock was actually released — if the lease
    /// sweep reaped the row first, it already accounted for it, and decrementing again would drift
    /// the gauge below the real in-flight count.
    pub(crate) async fn insert_test_run(
        &self,
        run: &NewTestRun,
        measurements: &[TestRunMeasurement],
    ) -> anyhow::Result<()> {
        let inserted = self
            .storage_manager
            .insert_test_run(run, measurements)
            .await?;
        if inserted.cleared_in_progress > 0 {
            PROMETHEUS_METRICS.inc_by(
                PrometheusMetric::TestrunsInProgress,
                -(inserted.cleared_in_progress as i64),
            );
        }
        Ok(())
    }

    /// The in-flight row for a node, i.e. what the orchestrator dispatched and is still waiting on.
    /// Read on submission to learn the kind and role a result must be recorded under, since the
    /// submission itself reports only the node and the address.
    ///
    /// `None` for a submission that arrives after its lease expired and the row was reaped.
    pub(crate) async fn get_testrun_in_progress(
        &self,
        node_id: NodeId,
    ) -> anyhow::Result<Option<TestRunInProgress>> {
        self.storage_manager
            .get_testrun_in_progress(node_id as i64)
            .await
    }

    /// Returns the number of rows currently in `testrun_in_progress`.
    pub(crate) async fn count_testruns_in_progress(&self) -> anyhow::Result<i64> {
        self.storage_manager.count_testruns_in_progress().await
    }

    /// Releases every in-flight lock whose lease has already expired, on the assumption that those
    /// runs will never report back. Decrements the `TestrunsInProgress` gauge by the number of rows
    /// actually cleared.
    ///
    /// Takes no timeout: the deadline lives on each row, stamped at dispatch from the budget of the
    /// kind being dispatched, so this sweep needs no knowledge of any kind's lease.
    pub(crate) async fn clear_expired_testruns_in_progress(&self) -> anyhow::Result<u64> {
        let cleared = self
            .storage_manager
            .clear_expired_testruns_in_progress(OffsetDateTime::now_utc())
            .await?;
        if cleared > 0 {
            PROMETHEUS_METRICS.inc_by(PrometheusMetric::TestrunsInProgress, -(cleared as i64));
        }
        Ok(cleared)
    }

    /// Atomically selects the most stale idle mixnode and marks it as having a test run in
    /// progress, with a lease of `lease_budget` from now.
    ///
    /// Staleness and the address rotation are evaluated for the `(stress, mixnode)` pairing alone,
    /// so no other kind's cadence disturbs this one. "Most stale" means: nodes that pairing has
    /// never tested come first, followed by those whose last run under it is oldest.
    ///
    /// `staleness_age` acts as a minimum-staleness gate: a node already tested by this pairing is
    /// only eligible if its last run completed more than `staleness_age` ago. Never-tested nodes
    /// are always eligible.
    ///
    /// Nodes with a row in `testrun_in_progress` are excluded whatever kind or role that row holds.
    /// Only nodes classified as `mixnode` or `mixnode_and_gateway` are eligible.
    ///
    /// Returns `None` if no eligible idle mixnode exists.
    pub(crate) async fn assign_next_mixnode_testrun(
        &self,
        staleness_age: Duration,
        lease_budget: Duration,
    ) -> anyhow::Result<Option<AssignedTestrun>> {
        let now = OffsetDateTime::now_utc();
        let last_tested_before = now - staleness_age;
        let expires_at = now + lease_budget;
        let assigned = self
            .storage_manager
            .assign_next_mixnode_testrun(now, last_tested_before, expires_at)
            .await?;
        if assigned.is_some() {
            PROMETHEUS_METRICS.inc(PrometheusMetric::TestrunsInProgress);
        }
        Ok(assigned)
    }

    /// Fetches a single completed test run with its measurements by its row id, or `None` if it
    /// has been evicted or never existed.
    pub(crate) async fn get_testrun_by_id(
        &self,
        id: i64,
    ) -> anyhow::Result<Option<CompletedTestRun>> {
        self.storage_manager.get_testrun_by_id(id).await
    }

    /// Fetches the newest completed run against a node, of any kind, with its measurements.
    /// `None` if the node has never been tested or its runs have all been evicted.
    pub(crate) async fn get_latest_testrun_for_node(
        &self,
        node_id: NodeId,
    ) -> anyhow::Result<Option<CompletedTestRun>> {
        self.storage_manager
            .get_latest_testrun_for_node(node_id as i64)
            .await
    }

    /// Fetches a node by its contract-assigned `node_id`, or `None` if the
    /// orchestrator has never observed a bond for it.
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

    /// Paginated list of completed test runs, with their measurements, ordered by
    /// `test_timestamp` descending (newest first), with the snapshot-consistent total row count.
    pub(crate) async fn get_testruns_paginated(
        &self,
        pagination: Pagination,
    ) -> anyhow::Result<(Vec<CompletedTestRun>, usize)> {
        let (test_results, total) = self
            .storage_manager
            .get_testruns_paginated(pagination.limit(), pagination.offset())
            .await?;

        Ok((test_results, total as usize))
    }

    /// Paginated list of completed test runs for a single node, with their measurements, ordered
    /// newest first, with the snapshot-consistent total row count. Backed by the
    /// `idx_testrun_node_id_timestamp` index. An unknown or never-tested `node_id` produces
    /// `(vec![], 0)` rather than an error.
    pub(crate) async fn get_testruns_for_node_paginated(
        &self,
        node_id: NodeId,
        pagination: Pagination,
    ) -> anyhow::Result<(Vec<CompletedTestRun>, usize)> {
        let (test_results, total) = self
            .storage_manager
            .get_testruns_for_node_paginated(
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

    /// Fetches every run of `test_kind` with `id > after_id`, with its measurements, ordered by id
    /// ascending.
    ///
    /// Used by the nym-api submission task to build the next batch of pending results. Ascending
    /// ordering lets the caller record the highest-id row as the new submission watermark once
    /// the batch is acknowledged. The kind filter keeps one stream from picking up the other's rows.
    pub(crate) async fn get_testruns_after(
        &self,
        test_kind: TestKind,
        after_id: i64,
    ) -> anyhow::Result<Vec<CompletedTestRun>> {
        self.storage_manager
            .get_testruns_after(test_kind, after_id)
            .await
    }

    /// Deletes all `testrun` rows older than `eviction_age` relative to the current time.
    ///
    /// Intended to be called periodically to keep the local database from growing unboundedly.
    /// Rows that are evicted are assumed to have already been submitted to the nym-api for
    /// persistent storage.
    ///
    /// Each run's measurement rows go with it, and any `node_test_state.last_testrun_id` pointing
    /// at an evicted row is set to `NULL` by the database. The pairing's `last_tested_at` survives,
    /// so an evicted result does not make the node read as never-tested.
    pub(crate) async fn evict_old_testruns(&self, eviction_age: Duration) -> anyhow::Result<u64> {
        let cutoff = OffsetDateTime::now_utc() - eviction_age;
        self.storage_manager.evict_old_testruns(cutoff).await
    }
}
