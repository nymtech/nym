// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::models::{NewNymNode, NewTestRun, NymNode};
use time::OffsetDateTime;

#[derive(Clone)]
pub(crate) struct StorageManager {
    pub(crate) connection_pool: sqlx::SqlitePool,
}

impl StorageManager {
    /// Inserts or updates multiple node records in a single transaction.
    ///
    /// For each node, if a row with the same `node_id` already exists, all fields except
    /// `identity_key` are updated — `identity_key` is intentionally left unchanged because
    /// a given `node_id` always corresponds to exactly one identity key and is never reassigned.
    ///
    /// Wrapping the entire batch in one transaction means SQLite performs a single WAL sync
    /// rather than one per row.
    pub(crate) async fn batch_insert_or_update_nym_nodes(
        &self,
        nodes: &[NewNymNode],
    ) -> anyhow::Result<()> {
        let mut tx = self.connection_pool.begin().await?;

        for node in nodes {
            sqlx::query!(
                r#"
                INSERT INTO nym_node (
                    node_id,
                    identity_key,
                    last_seen_bonded,
                    mixnet_socket_address,
                    noise_key,
                    sphinx_key,
                    key_rotation_id
                ) VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (node_id) DO UPDATE SET
                    last_seen_bonded      = excluded.last_seen_bonded,
                    mixnet_socket_address = excluded.mixnet_socket_address,
                    noise_key             = excluded.noise_key,
                    sphinx_key            = excluded.sphinx_key,
                    key_rotation_id       = excluded.key_rotation_id
                "#,
                node.node_id,
                node.identity_key,
                node.last_seen_bonded,
                node.mixnet_socket_address,
                node.noise_key,
                node.sphinx_key,
                node.key_rotation_id,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Inserts a completed test run and returns the auto-assigned row ID.
    pub(crate) async fn insert_test_run(&self, run: &NewTestRun) -> anyhow::Result<i64> {
        let id = sqlx::query!(
            r#"
            INSERT INTO testrun (
                test_type,
                test_timestamp,
                ingress_noise_handshake_us,
                egress_noise_handshake_us,
                packets_sent,
                packets_received,
                approximate_latency_us,
                packets_rtt_min_us,
                packets_rtt_mean_us,
                packets_rtt_max_us,
                packets_rtt_std_dev_us,
                sending_latency_min_us,
                sending_latency_mean_us,
                sending_latency_max_us,
                sending_latency_std_dev_us,
                received_duplicates,
                error
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            run.test_type,
            run.test_timestamp,
            run.ingress_noise_handshake_us,
            run.egress_noise_handshake_us,
            run.packets_sent,
            run.packets_received,
            run.approximate_latency_us,
            run.packets_rtt_min_us,
            run.packets_rtt_mean_us,
            run.packets_rtt_max_us,
            run.packets_rtt_std_dev_us,
            run.sending_latency_min_us,
            run.sending_latency_mean_us,
            run.sending_latency_max_us,
            run.sending_latency_std_dev_us,
            run.received_duplicates,
            run.error,
        )
        .execute(&self.connection_pool)
        .await?
        .last_insert_rowid();
        Ok(id)
    }

    /// Marks a node as having a test run in progress by inserting into `testrun_in_progress`.
    /// Returns an error if the node already has a run in progress (PRIMARY KEY conflict).
    #[cfg(test)]
    pub(crate) async fn mark_testrun_in_progress(
        &self,
        node_id: i64,
        started_at: OffsetDateTime,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            "INSERT INTO testrun_in_progress (node_id, started_at) VALUES (?, ?)",
            node_id,
            started_at,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Removes all in-progress markers with a `started_at` older than `cutoff`, on the
    /// assumption that those runs have timed out and will never complete.
    pub(crate) async fn clear_timed_out_testruns_in_progress(
        &self,
        cutoff: OffsetDateTime,
    ) -> anyhow::Result<u64> {
        let res = sqlx::query!(
            "DELETE FROM testrun_in_progress WHERE started_at < ?",
            cutoff,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(res.rows_affected())
    }

    /// Removes the in-progress marker for a node once its test run has completed or been abandoned.
    pub(crate) async fn clear_testrun_in_progress(&self, node_id: i64) -> anyhow::Result<()> {
        sqlx::query!("DELETE FROM testrun_in_progress WHERE node_id = ?", node_id,)
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    /// Atomically selects the most stale idle node and marks it as having a test run in progress.
    ///
    /// "Most stale" is defined as: nodes that have never been tested come first, followed by
    /// nodes whose last test run has the oldest timestamp.
    ///
    /// `last_tested_before` acts as a minimum-staleness gate: a node that has already been
    /// tested is only eligible if its last test run completed before this timestamp. Nodes
    /// that have never been tested are always eligible regardless of this value. The caller
    /// is expected to pass `now - min_test_interval`.
    ///
    /// `now` is recorded as the `started_at` timestamp on the resulting `testrun_in_progress`
    /// row. It is accepted as an argument rather than read from the clock internally so that
    /// callers can use a consistent timestamp across related operations.
    ///
    /// Nodes with a row in `testrun_in_progress` are excluded entirely.
    /// Nodes where `mixnet_socket_address`, `noise_key`, or `sphinx_key` is NULL are also
    /// excluded, as they lack the information required to perform a test.
    ///
    /// Returns `None` if no eligible idle node exists.
    pub(crate) async fn assign_next_testrun(
        &self,
        now: OffsetDateTime,
        last_tested_before: OffsetDateTime,
    ) -> anyhow::Result<Option<NymNode>> {
        // Starts a write (IMMEDIATE) transaction, to prevent issue when upgrading from a read one to a write one
        let mut tx = self.connection_pool.begin_with("BEGIN IMMEDIATE").await?;

        let node = sqlx::query_as::<_, NymNode>(
            r#"
            SELECT
                n.node_id,
                n.identity_key,
                n.last_seen_bonded,
                n.mixnet_socket_address,
                n.noise_key,
                n.sphinx_key,
                n.key_rotation_id,
                n.last_testrun
            FROM nym_node n
            LEFT JOIN testrun_in_progress tip ON tip.node_id = n.node_id
            LEFT JOIN testrun             tr  ON tr.id       = n.last_testrun
            WHERE tip.node_id IS NULL
              AND n.mixnet_socket_address IS NOT NULL
              AND n.noise_key IS NOT NULL
              AND n.sphinx_key IS NOT NULL
              AND (n.last_testrun IS NULL OR tr.test_timestamp < ?)
            ORDER BY tr.test_timestamp ASC NULLS FIRST
            LIMIT 1
            "#,
        )
        .bind(last_tested_before)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(ref node) = node {
            sqlx::query!(
                "INSERT INTO testrun_in_progress (node_id, started_at) VALUES (?, ?)",
                node.inner.node_id,
                now,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(node)
    }

    /// Deletes all `testrun` rows whose `test_timestamp` is older than `cutoff`.
    ///
    /// Intended to be called periodically with `now - eviction_age` as the cutoff to keep
    /// the local database from growing unboundedly. Rows that are evicted are assumed to
    /// have already been submitted to the nym-api for persistent storage.
    ///
    /// Any `nym_node.last_testrun` foreign key that pointed at an evicted row is automatically
    /// set to `NULL` by the database (`ON DELETE SET NULL`).
    pub(crate) async fn evict_old_testruns(&self, cutoff: OffsetDateTime) -> anyhow::Result<u64> {
        let res = sqlx::query!("DELETE FROM testrun WHERE test_timestamp < ?", cutoff)
            .execute(&self.connection_pool)
            .await?;
        Ok(res.rows_affected())
    }

    /// Updates `nym_node.last_testrun` to point at the given test run ID.
    pub(crate) async fn set_node_last_testrun(
        &self,
        node_id: i64,
        testrun_id: i64,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            "UPDATE nym_node SET last_testrun = ? WHERE node_id = ?",
            testrun_id,
            node_id,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::{NewNymNode, NewTestRun, TestType};
    use std::path::Path;
    use time::macros::datetime;

    async fn setup() -> StorageManager {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("failed to create in-memory SQLite pool");
        let migrations_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        sqlx::migrate::Migrator::new(migrations_path.as_path())
            .await
            .expect("failed to find migrations")
            .run(&pool)
            .await
            .expect("failed to run migrations");
        StorageManager {
            connection_pool: pool,
        }
    }

    fn node(id: i64, identity_key: &str) -> NewNymNode {
        NewNymNode {
            node_id: id,
            identity_key: identity_key.to_string(),
            last_seen_bonded: datetime!(2025-01-01 00:00:00 UTC),
            mixnet_socket_address: Some("1.2.3.4:1789".to_string()),
            noise_key: None,
            sphinx_key: None,
            key_rotation_id: None,
        }
    }

    fn minimal_test_run() -> NewTestRun {
        NewTestRun {
            test_type: TestType::Mixnode,
            test_timestamp: datetime!(2025-06-01 12:00:00 UTC),
            ingress_noise_handshake_us: None,
            egress_noise_handshake_us: None,
            packets_sent: 0,
            packets_received: 0,
            approximate_latency_us: None,
            packets_rtt_min_us: None,
            packets_rtt_mean_us: None,
            packets_rtt_max_us: None,
            packets_rtt_std_dev_us: None,
            sending_latency_min_us: None,
            sending_latency_mean_us: None,
            sending_latency_max_us: None,
            sending_latency_std_dev_us: None,
            received_duplicates: false,
            error: None,
        }
    }

    mod batch_insert_or_update_nym_nodes {
        use super::*;

        #[tokio::test]
        async fn inserts_multiple_nodes() {
            let db = setup().await;
            let nodes = vec![node(1, "key_a"), node(2, "key_b"), node(3, "key_c")];
            db.batch_insert_or_update_nym_nodes(&nodes).await.unwrap();

            let count = sqlx::query_scalar!("SELECT COUNT(*) FROM nym_node")
                .fetch_one(&db.connection_pool)
                .await
                .unwrap();
            assert_eq!(count, 3);
        }

        #[tokio::test]
        async fn updates_existing_nodes_in_batch() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_a")])
                .await
                .unwrap();

            let mut updated = node(1, "key_a");
            updated.mixnet_socket_address = Some("9.9.9.9:1789".to_string());
            updated.noise_key = Some("new_noise".to_string());

            let nodes = vec![updated, node(2, "key_b")];
            db.batch_insert_or_update_nym_nodes(&nodes).await.unwrap();

            let row = sqlx::query!(
                "SELECT mixnet_socket_address, noise_key FROM nym_node WHERE node_id = 1"
            )
            .fetch_one(&db.connection_pool)
            .await
            .unwrap();
            assert_eq!(row.mixnet_socket_address.as_deref(), Some("9.9.9.9:1789"));
            assert_eq!(row.noise_key.as_deref(), Some("new_noise"));

            let count = sqlx::query_scalar!("SELECT COUNT(*) FROM nym_node")
                .fetch_one(&db.connection_pool)
                .await
                .unwrap();
            assert_eq!(count, 2);
        }

        #[tokio::test]
        async fn empty_batch_is_noop() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[]).await.unwrap();

            let count = sqlx::query_scalar!("SELECT COUNT(*) FROM nym_node")
                .fetch_one(&db.connection_pool)
                .await
                .unwrap();
            assert_eq!(count, 0);
        }
    }

    mod insert_test_run {
        use super::*;

        #[tokio::test]
        async fn returns_sequential_ids() {
            let db = setup().await;
            let id1 = db.insert_test_run(&minimal_test_run()).await.unwrap();
            let id2 = db.insert_test_run(&minimal_test_run()).await.unwrap();
            assert!(id2 > id1);
        }

        #[tokio::test]
        async fn persists_fields() {
            let db = setup().await;
            let mut run = minimal_test_run();
            run.packets_sent = 100;
            run.packets_received = 95;
            run.received_duplicates = true;
            run.error = Some("timeout".to_string());
            let id = db.insert_test_run(&run).await.unwrap();

            let row = sqlx::query!(
                "SELECT packets_sent, packets_received, received_duplicates, error
                 FROM testrun WHERE id = ?",
                id
            )
            .fetch_one(&db.connection_pool)
            .await
            .unwrap();
            assert_eq!(row.packets_sent, 100);
            assert_eq!(row.packets_received, 95);
            assert!(row.received_duplicates);
            assert_eq!(row.error.as_deref(), Some("timeout"));
        }
    }

    mod set_node_last_testrun {
        use super::*;

        #[tokio::test]
        async fn links_run_to_node() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_a")])
                .await
                .unwrap();
            let run_id = db.insert_test_run(&minimal_test_run()).await.unwrap();
            db.set_node_last_testrun(1, run_id).await.unwrap();

            let row = sqlx::query!("SELECT last_testrun FROM nym_node WHERE node_id = 1")
                .fetch_one(&db.connection_pool)
                .await
                .unwrap();
            assert_eq!(row.last_testrun, Some(run_id));
        }
    }

    mod mark_testrun_in_progress {
        use super::*;

        #[tokio::test]
        async fn inserts_row() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_a")])
                .await
                .unwrap();
            db.mark_testrun_in_progress(1, datetime!(2025-06-01 10:00:00 UTC))
                .await
                .unwrap();

            let count =
                sqlx::query_scalar!("SELECT COUNT(*) FROM testrun_in_progress WHERE node_id = 1")
                    .fetch_one(&db.connection_pool)
                    .await
                    .unwrap();
            assert_eq!(count, 1);
        }

        #[tokio::test]
        async fn rejects_duplicate() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_a")])
                .await
                .unwrap();
            db.mark_testrun_in_progress(1, datetime!(2025-06-01 10:00:00 UTC))
                .await
                .unwrap();
            let result = db
                .mark_testrun_in_progress(1, datetime!(2025-06-01 11:00:00 UTC))
                .await;
            assert!(result.is_err());
        }
    }

    mod clear_testrun_in_progress {
        use super::*;

        #[tokio::test]
        async fn removes_row() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_a")])
                .await
                .unwrap();
            db.mark_testrun_in_progress(1, datetime!(2025-06-01 10:00:00 UTC))
                .await
                .unwrap();
            db.clear_testrun_in_progress(1).await.unwrap();

            let count =
                sqlx::query_scalar!("SELECT COUNT(*) FROM testrun_in_progress WHERE node_id = 1")
                    .fetch_one(&db.connection_pool)
                    .await
                    .unwrap();
            assert_eq!(count, 0);
        }
    }

    mod clear_timed_out_testruns_in_progress {
        use super::*;

        #[tokio::test]
        async fn removes_only_old_entries() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_a")])
                .await
                .unwrap();
            db.batch_insert_or_update_nym_nodes(&[node(2, "key_b")])
                .await
                .unwrap();
            db.mark_testrun_in_progress(1, datetime!(2025-06-01 08:00:00 UTC))
                .await
                .unwrap();
            db.mark_testrun_in_progress(2, datetime!(2025-06-01 12:00:00 UTC))
                .await
                .unwrap();

            // cutoff between the two timestamps
            db.clear_timed_out_testruns_in_progress(datetime!(2025-06-01 10:00:00 UTC))
                .await
                .unwrap();

            let remaining: Vec<i64> =
                sqlx::query_scalar!("SELECT node_id FROM testrun_in_progress ORDER BY node_id")
                    .fetch_all(&db.connection_pool)
                    .await
                    .unwrap();
            assert_eq!(remaining, vec![2]);
        }
    }

    mod evict_old_testruns {
        use super::*;

        #[tokio::test]
        async fn evicts_runs_older_than_cutoff() {
            let db = setup().await;
            let mut old_run = minimal_test_run();
            old_run.test_timestamp = datetime!(2025-01-01 00:00:00 UTC);
            let old_id = db.insert_test_run(&old_run).await.unwrap();

            let mut recent_run = minimal_test_run();
            recent_run.test_timestamp = datetime!(2025-06-01 12:00:00 UTC);
            let recent_id = db.insert_test_run(&recent_run).await.unwrap();

            db.evict_old_testruns(datetime!(2025-03-01 00:00:00 UTC))
                .await
                .unwrap();

            let ids: Vec<i64> = sqlx::query_scalar!("SELECT id FROM testrun ORDER BY id")
                .fetch_all(&db.connection_pool)
                .await
                .unwrap();
            assert!(!ids.contains(&old_id));
            assert!(ids.contains(&recent_id));
        }

        #[tokio::test]
        async fn preserves_runs_at_or_after_cutoff() {
            let db = setup().await;
            let mut run = minimal_test_run();
            run.test_timestamp = datetime!(2025-03-01 00:00:00 UTC);
            let id = db.insert_test_run(&run).await.unwrap();

            // cutoff is exactly at the run's timestamp — should NOT be evicted (strict <)
            db.evict_old_testruns(datetime!(2025-03-01 00:00:00 UTC))
                .await
                .unwrap();

            let count = sqlx::query_scalar!("SELECT COUNT(*) FROM testrun WHERE id = ?", id)
                .fetch_one(&db.connection_pool)
                .await
                .unwrap();
            assert_eq!(count, 1);
        }

        #[tokio::test]
        async fn nullifies_node_last_testrun_on_eviction() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_a")])
                .await
                .unwrap();

            let mut run = minimal_test_run();
            run.test_timestamp = datetime!(2025-01-01 00:00:00 UTC);
            let run_id = db.insert_test_run(&run).await.unwrap();
            db.set_node_last_testrun(1, run_id).await.unwrap();

            db.evict_old_testruns(datetime!(2025-06-01 00:00:00 UTC))
                .await
                .unwrap();

            let row = sqlx::query!("SELECT last_testrun FROM nym_node WHERE node_id = 1")
                .fetch_one(&db.connection_pool)
                .await
                .unwrap();
            assert!(row.last_testrun.is_none());
        }

        #[tokio::test]
        async fn does_nothing_when_no_old_runs() {
            let db = setup().await;
            db.insert_test_run(&minimal_test_run()).await.unwrap();

            // cutoff is well in the past — nothing should be evicted
            let result = db
                .evict_old_testruns(datetime!(2000-01-01 00:00:00 UTC))
                .await;
            assert!(result.is_ok());

            let count = sqlx::query_scalar!("SELECT COUNT(*) FROM testrun")
                .fetch_one(&db.connection_pool)
                .await
                .unwrap();
            assert_eq!(count, 1);
        }
    }

    mod assign_next_testrun {
        use super::*;

        // A far-future cutoff that effectively disables the staleness gate,
        // used in tests that are not concerned with that behaviour.
        fn no_staleness_gate() -> OffsetDateTime {
            datetime!(9999-12-31 23:59:59 UTC)
        }

        #[tokio::test]
        async fn returns_none_when_no_nodes() {
            let db = setup().await;
            let result = db
                .assign_next_testrun(datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate())
                .await
                .unwrap();
            assert!(result.is_none());
        }

        #[tokio::test]
        async fn returns_none_when_all_nodes_in_progress() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_a")])
                .await
                .unwrap();
            db.assign_next_testrun(datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate())
                .await
                .unwrap();

            let result = db
                .assign_next_testrun(datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate())
                .await
                .unwrap();
            assert!(result.is_none());
        }

        #[tokio::test]
        async fn inserts_in_progress_row() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_a")])
                .await
                .unwrap();
            let assigned = db
                .assign_next_testrun(datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate())
                .await
                .unwrap();
            assert!(assigned.is_some());

            let count =
                sqlx::query_scalar!("SELECT COUNT(*) FROM testrun_in_progress WHERE node_id = 1")
                    .fetch_one(&db.connection_pool)
                    .await
                    .unwrap();
            assert_eq!(count, 1);
        }

        #[tokio::test]
        async fn prefers_never_tested_node_over_stale_one() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_a")])
                .await
                .unwrap();
            db.batch_insert_or_update_nym_nodes(&[node(2, "key_b")])
                .await
                .unwrap();

            // give node 1 a completed test run
            let run_id = db.insert_test_run(&minimal_test_run()).await.unwrap();
            db.set_node_last_testrun(1, run_id).await.unwrap();

            // node 2 has never been tested — it should be picked first
            let assigned = db
                .assign_next_testrun(datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(assigned.inner.node_id, 2);
        }

        #[tokio::test]
        async fn prefers_older_testrun_over_newer_one() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_a")])
                .await
                .unwrap();
            db.batch_insert_or_update_nym_nodes(&[node(2, "key_b")])
                .await
                .unwrap();

            let mut old_run = minimal_test_run();
            old_run.test_timestamp = datetime!(2025-01-01 00:00:00 UTC);
            let old_id = db.insert_test_run(&old_run).await.unwrap();
            db.set_node_last_testrun(1, old_id).await.unwrap();

            let mut new_run = minimal_test_run();
            new_run.test_timestamp = datetime!(2025-06-01 12:00:00 UTC);
            let new_id = db.insert_test_run(&new_run).await.unwrap();
            db.set_node_last_testrun(2, new_id).await.unwrap();

            // node 1 has the older run — it should be picked
            let assigned = db
                .assign_next_testrun(datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(assigned.inner.node_id, 1);
        }

        #[tokio::test]
        async fn skips_node_already_in_progress() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_a")])
                .await
                .unwrap();
            db.batch_insert_or_update_nym_nodes(&[node(2, "key_b")])
                .await
                .unwrap();

            // both have no test run; node 1 is manually put in progress
            db.mark_testrun_in_progress(1, datetime!(2025-06-01 11:00:00 UTC))
                .await
                .unwrap();

            let assigned = db
                .assign_next_testrun(datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(assigned.inner.node_id, 2);
        }

        #[tokio::test]
        async fn skips_node_tested_too_recently() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_a")])
                .await
                .unwrap();

            let mut run = minimal_test_run();
            run.test_timestamp = datetime!(2025-06-01 12:00:00 UTC);
            let run_id = db.insert_test_run(&run).await.unwrap();
            db.set_node_last_testrun(1, run_id).await.unwrap();

            // cutoff is before the last test — node is not stale enough
            let result = db
                .assign_next_testrun(
                    datetime!(2025-06-01 13:00:00 UTC),
                    datetime!(2025-06-01 11:00:00 UTC),
                )
                .await
                .unwrap();
            assert!(result.is_none());
        }

        #[tokio::test]
        async fn returns_node_tested_sufficiently_long_ago() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_a")])
                .await
                .unwrap();

            let mut run = minimal_test_run();
            run.test_timestamp = datetime!(2025-06-01 12:00:00 UTC);
            let run_id = db.insert_test_run(&run).await.unwrap();
            db.set_node_last_testrun(1, run_id).await.unwrap();

            // cutoff is after the last test — node is eligible
            let assigned = db
                .assign_next_testrun(
                    datetime!(2025-06-01 14:00:00 UTC),
                    datetime!(2025-06-01 13:00:00 UTC),
                )
                .await
                .unwrap();
            assert!(assigned.is_some());
        }

        #[tokio::test]
        async fn never_tested_node_bypasses_staleness_gate() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_a")])
                .await
                .unwrap();
            db.batch_insert_or_update_nym_nodes(&[node(2, "key_b")])
                .await
                .unwrap();

            // node 1 was tested very recently
            let mut run = minimal_test_run();
            run.test_timestamp = datetime!(2025-06-01 12:00:00 UTC);
            let run_id = db.insert_test_run(&run).await.unwrap();
            db.set_node_last_testrun(1, run_id).await.unwrap();

            // cutoff is before node 1's last test — it is filtered out
            // node 2 has never been tested and must still be returned
            let assigned = db
                .assign_next_testrun(
                    datetime!(2025-06-01 13:00:00 UTC),
                    datetime!(2025-06-01 11:00:00 UTC),
                )
                .await
                .unwrap()
                .unwrap();
            assert_eq!(assigned.inner.node_id, 2);
        }
    }
}
