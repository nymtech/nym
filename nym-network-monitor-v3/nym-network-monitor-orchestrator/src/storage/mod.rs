// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::manager::StorageManager;
use crate::storage::models::{NewNymNode, NewTestRun, NymNode, TestRun, TestRunInProgress};
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

    /// Inserts a completed test run, updates the node's `last_testrun` pointer and
    /// clears the corresponding `testrun_in_progress` marker. The target node is
    /// taken from [`NewTestRun::node_id`].
    pub(crate) async fn insert_test_run(&self, run: &NewTestRun) -> anyhow::Result<()> {
        let node_id = run.node_id;
        let run_id = self.storage_manager.insert_test_run(run).await?;
        self.storage_manager
            .set_node_last_testrun(node_id, run_id)
            .await?;
        self.storage_manager
            .clear_testrun_in_progress(node_id)
            .await?;
        Ok(())
    }

    /// Removes all in-progress markers whose `started_at` is older than `timeout`, on the
    /// assumption that those runs have timed out and will never complete.
    pub(crate) async fn clear_timed_out_testruns_in_progress(
        &self,
        timeout: Duration,
    ) -> anyhow::Result<u64> {
        let cutoff = OffsetDateTime::now_utc() - timeout;
        self.storage_manager
            .clear_timed_out_testruns_in_progress(cutoff)
            .await
    }

    /// Atomically selects the most stale idle mixnode and marks it as having a test run in
    /// progress.
    ///
    /// "Most stale" is defined as: nodes that have never been tested come first, followed by
    /// nodes whose last test run has the oldest timestamp.
    ///
    /// `staleness_age` acts as a minimum-staleness gate: a node that has already been tested
    /// is only eligible if its last test run completed more than `staleness_age` ago. Nodes
    /// that have never been tested are always eligible regardless of this value.
    ///
    /// The current time is used as the `started_at` timestamp on the resulting
    /// `testrun_in_progress` row.
    ///
    /// Nodes with a row in `testrun_in_progress` are excluded entirely. Only nodes classified
    /// as `mixnode` or `mixnode_and_gateway` are eligible.
    ///
    /// Returns `None` if no eligible idle mixnode exists.
    pub(crate) async fn assign_next_mixnode_testrun(
        &self,
        staleness_age: Duration,
    ) -> anyhow::Result<Option<NymNode>> {
        let now = OffsetDateTime::now_utc();
        let last_tested_before = now - staleness_age;
        self.storage_manager
            .assign_next_mixnode_testrun(now, last_tested_before)
            .await
    }

    /// Fetches a single completed test run by its row id, or `None` if it has
    /// been evicted or never existed.
    pub(crate) async fn get_testrun_by_id(&self, id: i64) -> anyhow::Result<Option<TestRun>> {
        self.storage_manager.get_testrun_by_id(id).await
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

    /// Paginated list of completed test runs ordered by `test_timestamp`
    /// descending (newest first), with the snapshot-consistent total row count.
    pub(crate) async fn get_testruns_paginated(
        &self,
        pagination: Pagination,
    ) -> anyhow::Result<(Vec<TestRun>, usize)> {
        let (test_results, total) = self
            .storage_manager
            .get_testruns_paginated(pagination.limit(), pagination.offset())
            .await?;

        Ok((test_results, total as usize))
    }

    /// Paginated list of completed test runs for a single node, ordered newest
    /// first, with the snapshot-consistent total row count. Backed by the
    /// `idx_testrun_node_id_timestamp` index. An unknown or never-tested
    /// `node_id` produces `(vec![], 0)` rather than an error.
    pub(crate) async fn get_testruns_for_node_paginated(
        &self,
        node_id: NodeId,
        pagination: Pagination,
    ) -> anyhow::Result<(Vec<TestRun>, usize)> {
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

    /// Deletes all `testrun` rows older than `eviction_age` relative to the current time.
    ///
    /// Intended to be called periodically to keep the local database from growing unboundedly.
    /// Rows that are evicted are assumed to have already been submitted to the nym-api for
    /// persistent storage.
    ///
    /// Any `nym_node.last_testrun` foreign key that pointed at an evicted row is automatically
    /// set to `NULL` by the database (`ON DELETE SET NULL`).
    pub(crate) async fn evict_old_testruns(&self, eviction_age: Duration) -> anyhow::Result<u64> {
        let cutoff = OffsetDateTime::now_utc() - eviction_age;
        self.storage_manager.evict_old_testruns(cutoff).await
    }
}
