// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::models::{
    AssignedTestrun, AssignmentCandidate, AssignmentRequest, CompletedTestRun,
    GatewayLivenessTestRunRow, InsertedTestRun, MixnodeTestRunRow, NewTestRun, RefreshedNode,
    TestKind, TestRun, TestRunInProgress, duration_to_us, next_ip_to_test,
};
use nym_network_monitor_orchestrator_requests::models::{InterfaceMeasurement, RunMeasurements};
use sqlx::SqliteConnection;
use std::collections::HashMap;
use time::OffsetDateTime;

#[derive(Clone)]
pub(crate) struct StorageManager {
    pub(crate) connection_pool: sqlx::SqlitePool,
}

/// The nodes `kind` could assign right now, most overdue first, up to `limit` of them.
///
/// A candidate has a description (so it is completely described and still bonded), reports the role
/// `kind` probes, has no test of ANY kind in flight, and was either never measured by `kind` or last
/// measured before `last_tested_before`. Never-measured nodes come first, then the oldest
/// measurement. A dual-role node passes both role filters, so it is a candidate for every kind.
///
/// One plain query per role rather than one query with the role filter spliced or parameterised
/// in, so each reads as exactly what it selects. Both the assignment and its peek run this, which
/// is what keeps the two from judging different populations.
async fn select_candidates(
    conn: &mut SqliteConnection,
    kind: TestKind,
    last_tested_before: OffsetDateTime,
    limit: i64,
) -> anyhow::Result<Vec<AssignmentCandidate>> {
    let candidates = match kind {
        TestKind::MixnodeLiveness | TestKind::MixnodeStress => {
            sqlx::query_as!(
                AssignmentCandidate,
                r#"
                SELECT
                    d.node_id AS "node_id!",
                    b.identity_key,
                    d.mix_port,
                    d.announced_ips,
                    d.noise_key,
                    d.sphinx_key,
                    d.key_rotation_id,
                    d.clients_ws_port AS "clients_ws_port?",
                    s.last_tested_ip AS "last_tested_ip?",
                    s.last_tested_at AS "last_tested_at?: OffsetDateTime"
                FROM nym_node_description d
                JOIN nym_node_bond b              ON b.node_id = d.node_id
                LEFT JOIN testrun_in_progress tip ON tip.node_id = d.node_id
                LEFT JOIN node_test_state s       ON s.node_id = d.node_id AND s.test_kind = ?
                WHERE tip.node_id IS NULL
                  AND d.mixnode_enabled
                  AND (s.last_tested_at IS NULL OR s.last_tested_at < ?)
                ORDER BY s.last_tested_at ASC NULLS FIRST
                LIMIT ?
                "#,
                kind,
                last_tested_before,
                limit,
            )
            .fetch_all(conn)
            .await?
        }
        TestKind::GatewayLiveness => {
            sqlx::query_as!(
                AssignmentCandidate,
                r#"
                SELECT
                    d.node_id AS "node_id!",
                    b.identity_key,
                    d.mix_port,
                    d.announced_ips,
                    d.noise_key,
                    d.sphinx_key,
                    d.key_rotation_id,
                    d.clients_ws_port AS "clients_ws_port?",
                    s.last_tested_ip AS "last_tested_ip?",
                    s.last_tested_at AS "last_tested_at?: OffsetDateTime"
                FROM nym_node_description d
                JOIN nym_node_bond b              ON b.node_id = d.node_id
                LEFT JOIN testrun_in_progress tip ON tip.node_id = d.node_id
                LEFT JOIN node_test_state s       ON s.node_id = d.node_id AND s.test_kind = ?
                WHERE tip.node_id IS NULL
                  AND d.gateway_enabled
                  AND (s.last_tested_at IS NULL OR s.last_tested_at < ?)
                ORDER BY s.last_tested_at ASC NULLS FIRST
                LIMIT ?
                "#,
                kind,
                last_tested_before,
                limit,
            )
            .fetch_all(conn)
            .await?
        }
    };

    Ok(candidates)
}

/// Writes one `mixnode_liveness` run into its results table, returning the id it was stored under.
async fn insert_mixnode_liveness_testrun(
    conn: &mut SqliteConnection,
    run: &NewTestRun,
    mix_forwarding: &InterfaceMeasurement,
) -> anyhow::Result<i64> {
    let rtt = mix_forwarding.packets_statistics;
    let id = sqlx::query!(
        r#"
        INSERT INTO mixnode_liveness_testrun (
            node_id,
            tested_address,
            test_timestamp,
            time_taken_us,
            error,
            mix_forwarding_ingress_noise_handshake_us,
            mix_forwarding_egress_noise_handshake_us,
            mix_forwarding_sphinx_packet_delay_us,
            mix_forwarding_packets_sent,
            mix_forwarding_packets_received,
            mix_forwarding_approximate_latency_us,
            mix_forwarding_packets_rtt_min_us,
            mix_forwarding_packets_rtt_mean_us,
            mix_forwarding_packets_rtt_median_us,
            mix_forwarding_packets_rtt_max_us,
            mix_forwarding_packets_rtt_std_dev_us,
            mix_forwarding_received_duplicates
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        run.node_id,
        run.tested_address,
        run.test_timestamp,
        run.time_taken_us,
        run.error,
        mix_forwarding.ingress_noise_handshake.map(duration_to_us),
        mix_forwarding.egress_noise_handshake.map(duration_to_us),
        duration_to_us(mix_forwarding.sphinx_packet_delay),
        mix_forwarding.packets_sent as i64,
        mix_forwarding.packets_received as i64,
        mix_forwarding.approximate_latency.map(duration_to_us),
        rtt.map(|stats| duration_to_us(stats.minimum)),
        rtt.map(|stats| duration_to_us(stats.mean)),
        rtt.map(|stats| duration_to_us(stats.median)),
        rtt.map(|stats| duration_to_us(stats.maximum)),
        rtt.map(|stats| duration_to_us(stats.standard_deviation)),
        mix_forwarding.received_duplicates,
    )
    .execute(conn)
    .await?
    .last_insert_rowid();

    Ok(id)
}

/// Writes one `gateway_liveness` run into its results table, returning the id it was stored under.
async fn insert_gateway_liveness_testrun(
    conn: &mut SqliteConnection,
    run: &NewTestRun,
    client_ingest: &InterfaceMeasurement,
    client_delivery: &InterfaceMeasurement,
) -> anyhow::Result<i64> {
    let ingest_rtt = client_ingest.packets_statistics;
    let delivery_rtt = client_delivery.packets_statistics;
    let id = sqlx::query!(
        r#"
        INSERT INTO gateway_liveness_testrun (
            node_id,
            tested_address,
            test_timestamp,
            time_taken_us,
            error,
            client_ingest_ingress_noise_handshake_us,
            client_ingest_egress_noise_handshake_us,
            client_ingest_sphinx_packet_delay_us,
            client_ingest_packets_sent,
            client_ingest_packets_received,
            client_ingest_approximate_latency_us,
            client_ingest_packets_rtt_min_us,
            client_ingest_packets_rtt_mean_us,
            client_ingest_packets_rtt_median_us,
            client_ingest_packets_rtt_max_us,
            client_ingest_packets_rtt_std_dev_us,
            client_ingest_received_duplicates,
            client_delivery_ingress_noise_handshake_us,
            client_delivery_egress_noise_handshake_us,
            client_delivery_sphinx_packet_delay_us,
            client_delivery_packets_sent,
            client_delivery_packets_received,
            client_delivery_approximate_latency_us,
            client_delivery_packets_rtt_min_us,
            client_delivery_packets_rtt_mean_us,
            client_delivery_packets_rtt_median_us,
            client_delivery_packets_rtt_max_us,
            client_delivery_packets_rtt_std_dev_us,
            client_delivery_received_duplicates
        ) VALUES (
            ?, ?, ?, ?, ?,
            ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
            ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
        )
        "#,
        run.node_id,
        run.tested_address,
        run.test_timestamp,
        run.time_taken_us,
        run.error,
        client_ingest.ingress_noise_handshake.map(duration_to_us),
        client_ingest.egress_noise_handshake.map(duration_to_us),
        duration_to_us(client_ingest.sphinx_packet_delay),
        client_ingest.packets_sent as i64,
        client_ingest.packets_received as i64,
        client_ingest.approximate_latency.map(duration_to_us),
        ingest_rtt.map(|stats| duration_to_us(stats.minimum)),
        ingest_rtt.map(|stats| duration_to_us(stats.mean)),
        ingest_rtt.map(|stats| duration_to_us(stats.median)),
        ingest_rtt.map(|stats| duration_to_us(stats.maximum)),
        ingest_rtt.map(|stats| duration_to_us(stats.standard_deviation)),
        client_ingest.received_duplicates,
        client_delivery.ingress_noise_handshake.map(duration_to_us),
        client_delivery.egress_noise_handshake.map(duration_to_us),
        duration_to_us(client_delivery.sphinx_packet_delay),
        client_delivery.packets_sent as i64,
        client_delivery.packets_received as i64,
        client_delivery.approximate_latency.map(duration_to_us),
        delivery_rtt.map(|stats| duration_to_us(stats.minimum)),
        delivery_rtt.map(|stats| duration_to_us(stats.mean)),
        delivery_rtt.map(|stats| duration_to_us(stats.median)),
        delivery_rtt.map(|stats| duration_to_us(stats.maximum)),
        delivery_rtt.map(|stats| duration_to_us(stats.standard_deviation)),
        client_delivery.received_duplicates,
    )
    .execute(conn)
    .await?
    .last_insert_rowid();

    Ok(id)
}

/// Writes one `mixnode_stress` run into its results table, returning the id it was stored under.
async fn insert_mixnode_stress_testrun(
    conn: &mut SqliteConnection,
    run: &NewTestRun,
    mix_forwarding: &InterfaceMeasurement,
) -> anyhow::Result<i64> {
    let rtt = mix_forwarding.packets_statistics;
    let id = sqlx::query!(
        r#"
        INSERT INTO mixnode_stress_testrun (
            node_id,
            tested_address,
            test_timestamp,
            time_taken_us,
            error,
            mix_forwarding_ingress_noise_handshake_us,
            mix_forwarding_egress_noise_handshake_us,
            mix_forwarding_sphinx_packet_delay_us,
            mix_forwarding_packets_sent,
            mix_forwarding_packets_received,
            mix_forwarding_approximate_latency_us,
            mix_forwarding_packets_rtt_min_us,
            mix_forwarding_packets_rtt_mean_us,
            mix_forwarding_packets_rtt_median_us,
            mix_forwarding_packets_rtt_max_us,
            mix_forwarding_packets_rtt_std_dev_us,
            mix_forwarding_received_duplicates
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        run.node_id,
        run.tested_address,
        run.test_timestamp,
        run.time_taken_us,
        run.error,
        mix_forwarding.ingress_noise_handshake.map(duration_to_us),
        mix_forwarding.egress_noise_handshake.map(duration_to_us),
        duration_to_us(mix_forwarding.sphinx_packet_delay),
        mix_forwarding.packets_sent as i64,
        mix_forwarding.packets_received as i64,
        mix_forwarding.approximate_latency.map(duration_to_us),
        rtt.map(|stats| duration_to_us(stats.minimum)),
        rtt.map(|stats| duration_to_us(stats.mean)),
        rtt.map(|stats| duration_to_us(stats.median)),
        rtt.map(|stats| duration_to_us(stats.maximum)),
        rtt.map(|stats| duration_to_us(stats.standard_deviation)),
        mix_forwarding.received_duplicates,
    )
    .execute(conn)
    .await?
    .last_insert_rowid();

    Ok(id)
}

/// Every `mixnode_liveness` run with an id above `after_id`, oldest id first.
async fn get_mixnode_liveness_testruns_after(
    conn: &mut SqliteConnection,
    after_id: i64,
) -> anyhow::Result<Vec<CompletedTestRun>> {
    let rows = sqlx::query_as!(
        MixnodeTestRunRow,
        r#"
        SELECT
            id,
            node_id,
            tested_address,
            test_timestamp AS "test_timestamp: OffsetDateTime",
            time_taken_us,
            error,
            mix_forwarding_ingress_noise_handshake_us,
            mix_forwarding_egress_noise_handshake_us,
            mix_forwarding_sphinx_packet_delay_us,
            mix_forwarding_packets_sent,
            mix_forwarding_packets_received,
            mix_forwarding_approximate_latency_us,
            mix_forwarding_packets_rtt_min_us,
            mix_forwarding_packets_rtt_mean_us,
            mix_forwarding_packets_rtt_median_us,
            mix_forwarding_packets_rtt_max_us,
            mix_forwarding_packets_rtt_std_dev_us,
            mix_forwarding_received_duplicates
        FROM mixnode_liveness_testrun
        WHERE id > ?
        ORDER BY id ASC
        "#,
        after_id
    )
    .fetch_all(conn)
    .await?;

    Ok(rows
        .into_iter()
        .map(MixnodeTestRunRow::into_mixnode_liveness)
        .collect())
}

/// Every `gateway_liveness` run with an id above `after_id`, oldest id first.
async fn get_gateway_liveness_testruns_after(
    conn: &mut SqliteConnection,
    after_id: i64,
) -> anyhow::Result<Vec<CompletedTestRun>> {
    let rows = sqlx::query_as!(
        GatewayLivenessTestRunRow,
        r#"
        SELECT
            id,
            node_id,
            tested_address,
            test_timestamp AS "test_timestamp: OffsetDateTime",
            time_taken_us,
            error,
            client_ingest_ingress_noise_handshake_us,
            client_ingest_egress_noise_handshake_us,
            client_ingest_sphinx_packet_delay_us,
            client_ingest_packets_sent,
            client_ingest_packets_received,
            client_ingest_approximate_latency_us,
            client_ingest_packets_rtt_min_us,
            client_ingest_packets_rtt_mean_us,
            client_ingest_packets_rtt_median_us,
            client_ingest_packets_rtt_max_us,
            client_ingest_packets_rtt_std_dev_us,
            client_ingest_received_duplicates,
            client_delivery_ingress_noise_handshake_us,
            client_delivery_egress_noise_handshake_us,
            client_delivery_sphinx_packet_delay_us,
            client_delivery_packets_sent,
            client_delivery_packets_received,
            client_delivery_approximate_latency_us,
            client_delivery_packets_rtt_min_us,
            client_delivery_packets_rtt_mean_us,
            client_delivery_packets_rtt_median_us,
            client_delivery_packets_rtt_max_us,
            client_delivery_packets_rtt_std_dev_us,
            client_delivery_received_duplicates
        FROM gateway_liveness_testrun
        WHERE id > ?
        ORDER BY id ASC
        "#,
        after_id
    )
    .fetch_all(conn)
    .await?;

    Ok(rows
        .into_iter()
        .map(GatewayLivenessTestRunRow::into_gateway_liveness)
        .collect())
}

/// Every `mixnode_stress` run with an id above `after_id`, oldest id first.
async fn get_mixnode_stress_testruns_after(
    conn: &mut SqliteConnection,
    after_id: i64,
) -> anyhow::Result<Vec<CompletedTestRun>> {
    let rows = sqlx::query_as!(
        MixnodeTestRunRow,
        r#"
        SELECT
            id,
            node_id,
            tested_address,
            test_timestamp AS "test_timestamp: OffsetDateTime",
            time_taken_us,
            error,
            mix_forwarding_ingress_noise_handshake_us,
            mix_forwarding_egress_noise_handshake_us,
            mix_forwarding_sphinx_packet_delay_us,
            mix_forwarding_packets_sent,
            mix_forwarding_packets_received,
            mix_forwarding_approximate_latency_us,
            mix_forwarding_packets_rtt_min_us,
            mix_forwarding_packets_rtt_mean_us,
            mix_forwarding_packets_rtt_median_us,
            mix_forwarding_packets_rtt_max_us,
            mix_forwarding_packets_rtt_std_dev_us,
            mix_forwarding_received_duplicates
        FROM mixnode_stress_testrun
        WHERE id > ?
        ORDER BY id ASC
        "#,
        after_id
    )
    .fetch_all(conn)
    .await?;

    Ok(rows
        .into_iter()
        .map(MixnodeTestRunRow::into_mixnode_stress)
        .collect())
}

impl StorageManager {
    /// Records what one refresh learned, in a single transaction: every bond it read, the
    /// description of every node that answered completely, and the removal of the descriptions of
    /// nodes it did not see bonded.
    ///
    /// A bond is upserted with the refresh's `seen_at` and never has its `identity_key` changed,
    /// since a `node_id` always maps to exactly one identity. A description replaces the previous
    /// one whole. A node that did not answer keeps its previous description, so a merely slow node
    /// stays testable.
    ///
    /// Every bond this refresh read carries `seen_at`, so a bond with an older `last_seen_bonded`
    /// belongs to a node the contract no longer lists. Its description is deleted, which makes it
    /// ineligible for every kind, while its bond stays for the read surface. Only ever called after
    /// a successful contract read, so a failed one can never delete anything.
    pub(crate) async fn store_refresh(
        &self,
        nodes: &[RefreshedNode],
        seen_at: OffsetDateTime,
    ) -> anyhow::Result<()> {
        let mut tx = self.connection_pool.begin().await?;

        for node in nodes {
            let bond = &node.bond;
            sqlx::query!(
                r#"
                INSERT INTO nym_node_bond (node_id, identity_key, last_seen_bonded)
                VALUES (?, ?, ?)
                ON CONFLICT (node_id) DO UPDATE SET
                    last_seen_bonded = excluded.last_seen_bonded
                "#,
                bond.node_id,
                bond.identity_key,
                bond.last_seen_bonded,
            )
            .execute(&mut *tx)
            .await?;

            let Some(description) = &node.description else {
                continue;
            };
            sqlx::query!(
                r#"
                INSERT INTO nym_node_description (
                    node_id,
                    mix_port,
                    announced_ips,
                    noise_key,
                    sphinx_key,
                    key_rotation_id,
                    mixnode_enabled,
                    gateway_enabled,
                    clients_ws_port
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (node_id) DO UPDATE SET
                    mix_port        = excluded.mix_port,
                    announced_ips   = excluded.announced_ips,
                    noise_key       = excluded.noise_key,
                    sphinx_key      = excluded.sphinx_key,
                    key_rotation_id = excluded.key_rotation_id,
                    mixnode_enabled = excluded.mixnode_enabled,
                    gateway_enabled = excluded.gateway_enabled,
                    clients_ws_port = excluded.clients_ws_port
                "#,
                description.node_id,
                description.mix_port,
                description.announced_ips,
                description.noise_key,
                description.sphinx_key,
                description.key_rotation_id,
                description.mixnode_enabled,
                description.gateway_enabled,
                description.clients_ws_port,
            )
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query!(
            r#"
            DELETE FROM nym_node_description
            WHERE node_id IN (SELECT node_id FROM nym_node_bond WHERE last_seen_bonded < ?)
            "#,
            seen_at,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Persists a completed test run in its kind's results table, together with that kind's work
    /// state and the release of the node's in-flight lock. All three in ONE transaction, so a node
    /// is never left locked by a run that was already recorded.
    ///
    /// The table is chosen by the shape of the measurements, which the submission path has already
    /// checked against the kind the orchestrator dispatched. The kind's rotation pointer is
    /// deliberately not touched here: it belongs to the assignment, which advances it when the work
    /// is handed out so that an abandoned run still moves the node onto its next address.
    pub(crate) async fn insert_test_run(
        &self,
        run: &NewTestRun,
        measurements: &RunMeasurements,
    ) -> anyhow::Result<InsertedTestRun> {
        let mut tx = self.connection_pool.begin().await?;

        let id = match measurements {
            RunMeasurements::MixnodeLiveness { mix_forwarding } => {
                insert_mixnode_liveness_testrun(&mut tx, run, mix_forwarding).await?
            }
            RunMeasurements::GatewayLiveness {
                client_ingest,
                client_delivery,
            } => {
                insert_gateway_liveness_testrun(&mut tx, run, client_ingest, client_delivery)
                    .await?
            }
            RunMeasurements::MixnodeStress { mix_forwarding } => {
                insert_mixnode_stress_testrun(&mut tx, run, mix_forwarding).await?
            }
        };

        let kind = TestKind::from(measurements.kind());
        sqlx::query!(
            r#"
            INSERT INTO node_test_state (node_id, test_kind, last_tested_at)
            VALUES (?, ?, ?)
            ON CONFLICT (node_id, test_kind) DO UPDATE SET
                last_tested_at = excluded.last_tested_at
            "#,
            run.node_id,
            kind,
            run.test_timestamp,
        )
        .execute(&mut *tx)
        .await?;

        let cleared_in_progress = sqlx::query!(
            "DELETE FROM testrun_in_progress WHERE node_id = ?",
            run.node_id
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();

        tx.commit().await?;
        Ok(InsertedTestRun {
            id,
            cleared_in_progress,
        })
    }

    /// Marks a node as having a test run in progress by inserting into `testrun_in_progress`.
    /// Returns an error if the node already has a run in progress (PRIMARY KEY conflict).
    #[cfg(test)]
    pub(crate) async fn mark_testrun_in_progress(
        &self,
        node_id: i64,
        started_at: OffsetDateTime,
        expires_at: OffsetDateTime,
        test_kind: TestKind,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO testrun_in_progress (node_id, started_at, expires_at, test_kind)
            VALUES (?, ?, ?, ?)
            "#,
            node_id,
            started_at,
            expires_at,
            test_kind,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Reads the in-flight row for a node, which is the authoritative record of what the
    /// orchestrator dispatched: the submission carries only the node and the address, so the kind
    /// a result is stored under comes from here rather than from the agent.
    ///
    /// `None` once the lease has expired and the sweep has reaped the row, i.e. for a late
    /// submission.
    pub(crate) async fn get_testrun_in_progress(
        &self,
        node_id: i64,
    ) -> anyhow::Result<Option<TestRunInProgress>> {
        let row = sqlx::query_as::<_, TestRunInProgress>(
            "SELECT * FROM testrun_in_progress WHERE node_id = ?",
        )
        .bind(node_id)
        .fetch_optional(&self.connection_pool)
        .await?;
        Ok(row)
    }

    /// Releases every in-flight lock whose lease has run out as of `now`, on the assumption that
    /// those runs will never report back.
    ///
    /// Each row is judged by the deadline stamped on it at dispatch rather than by a cutoff derived
    /// from one global timeout, which is what lets kinds with different lease budgets expire on
    /// their own schedules: under a shared cutoff, a long-leased run would be reaped - and its node
    /// handed to a second agent - while the first agent was still legitimately working on it.
    ///
    /// The comparison is strict, so a lease expiring exactly at `now` survives until the next
    /// sweep, matching the result eviction sweep.
    ///
    /// Reports how many rows each kind lost, because that is the signal that a kind's lease budget
    /// is too short for the work it covers, and a total would hide it: liveness leases are minutes
    /// shorter than stress ones, so the two expire at very different rates even when both are
    /// healthy. Counted before the delete, in the same transaction, since the delete itself reports
    /// only a total.
    pub(crate) async fn clear_expired_testruns_in_progress(
        &self,
        now: OffsetDateTime,
    ) -> anyhow::Result<HashMap<TestKind, u64>> {
        let mut tx = self.connection_pool.begin_with("BEGIN IMMEDIATE").await?;

        let expiring = sqlx::query_as::<_, (TestKind, i64)>(
            "SELECT test_kind, COUNT(*) FROM testrun_in_progress WHERE expires_at < ? GROUP BY test_kind",
        )
        .bind(now)
        .fetch_all(&mut *tx)
        .await?;

        sqlx::query!("DELETE FROM testrun_in_progress WHERE expires_at < ?", now,)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(expiring
            .into_iter()
            .map(|(kind, count)| (kind, count as u64))
            .collect())
    }

    /// Returns the number of rows currently in `testrun_in_progress` — i.e. the number of
    /// test runs that have been assigned to an agent but not yet submitted back.
    pub(crate) async fn count_testruns_in_progress(&self) -> anyhow::Result<i64> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM testrun_in_progress")
            .fetch_one(&self.connection_pool)
            .await?;
        Ok(total)
    }

    /// The same count broken down by kind, for the per-kind in-flight gauges. Kinds with no rows are
    /// absent from the map rather than present as zero, so a caller publishing gauges has to decide
    /// what an absent kind means - it means zero.
    pub(crate) async fn count_testruns_in_progress_by_kind(
        &self,
    ) -> anyhow::Result<HashMap<TestKind, i64>> {
        let counts = sqlx::query_as::<_, (TestKind, i64)>(
            "SELECT test_kind, COUNT(*) FROM testrun_in_progress GROUP BY test_kind",
        )
        .fetch_all(&self.connection_pool)
        .await?;

        Ok(counts.into_iter().collect())
    }

    /// Atomically selects the most stale idle nodes eligible for one kind and marks each of them as
    /// having a test run in progress.
    ///
    /// Staleness, the rotation pointer and the resulting locks are all read and written for the
    /// requested kind alone, so no other kind's cadence can disturb this one. A stress request asks
    /// for one target; a liveness request asks for up to its kind's wave size, and the returned
    /// targets form one wave.
    ///
    /// "Most stale" is defined as: nodes this kind has never tested come first, followed by those
    /// whose last run under it has the oldest timestamp. [`AssignmentRequest::last_tested_before`]
    /// acts as a minimum-staleness gate that never-tested nodes bypass.
    ///
    /// Eligibility is that of [`select_candidates`]. A node whose in-flight row has just cleared is
    /// immediately eligible for another kind, the per-node lock being the whole of the mutual
    /// exclusion between kinds.
    ///
    /// Returns an empty vector when no eligible idle node exists. A target whose stored addresses
    /// cannot be parsed is dropped from the wave rather than failing the assignment.
    pub(crate) async fn assign_next_testruns(
        &self,
        request: &AssignmentRequest,
    ) -> anyhow::Result<Vec<AssignedTestrun>> {
        // Starts a write (IMMEDIATE) transaction, to prevent issue when upgrading from a read one to a write one
        let mut tx = self.connection_pool.begin_with("BEGIN IMMEDIATE").await?;

        let candidates = select_candidates(
            &mut tx,
            request.kind,
            request.last_tested_before,
            request.wave_size as i64,
        )
        .await?;

        let mut assigned = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            // rotate onto the next announced address of that node, following this kind's own
            // pointer. a description always carries a non-empty announced set, so this can only be
            // `None` for a row whose stored addresses are corrupt, and dropping that one target keeps
            // the rest of the wave assignable
            let announced = candidate.announced_ips();
            let Some(tested_ip) = next_ip_to_test(&announced, candidate.last_tested_ip.as_deref())
            else {
                continue;
            };

            // advance the rotation pointer here rather than on result submission, so that runs which
            // never report back still move the node onto its next address
            let node_id = candidate.node_id;
            let stored_tested_ip = tested_ip.to_string();
            sqlx::query!(
                r#"
                INSERT INTO node_test_state (node_id, test_kind, last_tested_ip)
                VALUES (?, ?, ?)
                ON CONFLICT (node_id, test_kind) DO UPDATE SET
                    last_tested_ip = excluded.last_tested_ip
                "#,
                node_id,
                request.kind,
                stored_tested_ip,
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                r#"
                INSERT INTO testrun_in_progress (node_id, started_at, expires_at, test_kind)
                VALUES (?, ?, ?, ?)
                "#,
                node_id,
                request.now,
                request.expires_at,
                request.kind,
            )
            .execute(&mut *tx)
            .await?;

            assigned.push(AssignedTestrun {
                node: candidate,
                tested_ip,
            });
        }

        tx.commit().await?;
        Ok(assigned)
    }

    /// The node this kind would assign next, or `None` when it has nothing eligible.
    ///
    /// Runs the very query the assignment runs, at `LIMIT 1`, so it reports the node the assignment
    /// would take first and can never judge a different population.
    pub(crate) async fn peek_next_candidate(
        &self,
        kind: TestKind,
        last_tested_before: OffsetDateTime,
    ) -> anyhow::Result<Option<AssignmentCandidate>> {
        let mut conn = self.connection_pool.acquire().await?;
        let candidates = select_candidates(&mut conn, kind, last_tested_before, 1).await?;
        Ok(candidates.into_iter().next())
    }

    /// Fetches a single `testrun` row by its primary key, together with its measurements.
    ///
    /// Returns `None` if no row with that id exists.
    pub(crate) async fn get_testrun_by_id(
        &self,
        id: i64,
    ) -> anyhow::Result<Option<CompletedTestRun>> {
        let mut tx = self.connection_pool.begin().await?;

        let run = sqlx::query_as::<_, TestRun>("SELECT * FROM testrun WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;

        let completed = match run {
            None => None,
            Some(run) => complete_runs(&mut tx, vec![run]).await?.pop(),
        };

        tx.commit().await?;
        Ok(completed)
    }

    /// Fetches the newest completed run against a node, together with its measurements, or `None`
    /// if the node has never been tested (or its runs have all been evicted).
    ///
    /// Reads the run table directly rather than following a `node_test_state` pointer, because a
    /// node may hold one pointer per kind and the read surface wants the newest run
    /// of any of them. Backed by the `idx_testrun_node_id_timestamp` index.
    pub(crate) async fn get_latest_testrun_for_node(
        &self,
        node_id: i64,
    ) -> anyhow::Result<Option<CompletedTestRun>> {
        let mut tx = self.connection_pool.begin().await?;

        let run = sqlx::query_as::<_, TestRun>(
            "SELECT * FROM testrun WHERE node_id = ? ORDER BY test_timestamp DESC LIMIT 1",
        )
        .bind(node_id)
        .fetch_optional(&mut *tx)
        .await?;

        let completed = match run {
            None => None,
            Some(run) => complete_runs(&mut tx, vec![run]).await?.pop(),
        };

        tx.commit().await?;
        Ok(completed)
    }

    /// Fetches a single `nym_node` row by its `node_id`.
    ///
    /// Returns `None` if the orchestrator has never seen a bond for this node.
    pub(crate) async fn get_nym_node_by_id(&self, node_id: i64) -> anyhow::Result<Option<NymNode>> {
        let row = sqlx::query_as::<_, NymNode>("SELECT * FROM nym_node WHERE node_id = ?")
            .bind(node_id)
            .fetch_optional(&self.connection_pool)
            .await?;
        Ok(row)
    }

    /// Fetches a page of `testrun` rows filtered to a single `node_id`, ordered by
    /// `test_timestamp` descending (newest first), together with the total number of rows
    /// for that node (used to populate `PagedResult::total`).
    ///
    /// Backed by the `idx_testrun_node_id_timestamp` index.
    ///
    /// `limit` and `offset` translate directly to SQL `LIMIT` / `OFFSET`; the caller is
    /// expected to derive them from the public pagination contract as
    /// `limit = size` and `offset = page * size`.
    ///
    /// The page, its measurements and the total count are fetched inside a single transaction so
    /// that the `total` is consistent with the rows returned and no run loses its measurements to
    /// a concurrent eviction mid-read.
    pub(crate) async fn get_testruns_for_node_paginated(
        &self,
        node_id: i64,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(Vec<CompletedTestRun>, i64)> {
        let mut tx = self.connection_pool.begin().await?;

        let runs = sqlx::query_as::<_, TestRun>(
            "SELECT * FROM testrun WHERE node_id = ? ORDER BY test_timestamp DESC LIMIT ? OFFSET ?",
        )
        .bind(node_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *tx)
        .await?;

        let completed = complete_runs(&mut tx, runs).await?;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM testrun WHERE node_id = ?")
            .bind(node_id)
            .fetch_one(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok((completed, total))
    }

    /// Fetches a page of `testrun` rows, ordered by `test_timestamp` descending (newest first),
    /// together with the total number of rows in the table (used to populate
    /// `PagedResult::total`).
    ///
    /// `limit` and `offset` translate directly to SQL `LIMIT` / `OFFSET`; the caller is
    /// expected to derive them from the public pagination contract as
    /// `limit = size` and `offset = page * size`.
    ///
    /// The page, its measurements and the total count share one transaction (no tearing if another
    /// writer commits in between).
    pub(crate) async fn get_testruns_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(Vec<CompletedTestRun>, i64)> {
        let mut tx = self.connection_pool.begin().await?;

        let runs = sqlx::query_as::<_, TestRun>(
            "SELECT * FROM testrun ORDER BY test_timestamp DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *tx)
        .await?;

        let completed = complete_runs(&mut tx, runs).await?;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM testrun")
            .fetch_one(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok((completed, total))
    }

    /// Fetches a page of `nym_node` rows, ordered by `node_id` ascending, together with the
    /// total number of rows in the table (used to populate `PagedResult::total`).
    ///
    /// `limit` and `offset` translate directly to SQL `LIMIT` / `OFFSET`; the caller is
    /// expected to derive them from the public pagination contract as
    /// `limit = size` and `offset = page * size`.
    ///
    /// The page and total count are fetched inside a single transaction so that the `total`
    /// is consistent with the rows returned (no tearing if another writer commits in between).
    pub(crate) async fn get_nym_nodes_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(Vec<NymNode>, i64)> {
        let mut tx = self.connection_pool.begin().await?;

        let rows = sqlx::query_as::<_, NymNode>(
            "SELECT * FROM nym_node ORDER BY node_id ASC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *tx)
        .await?;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM nym_node")
            .fetch_one(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok((rows, total))
    }

    /// Fetches a page of `testrun_in_progress` rows, ordered from oldest `started_at` to
    /// newest (so stale/hung runs surface first), together with the total number of rows in
    /// the table (used to populate `PagedResult::total`).
    ///
    /// `limit` and `offset` translate directly to SQL `LIMIT` / `OFFSET`; the caller is
    /// expected to derive them from the public pagination contract as
    /// `limit = size` and `offset = page * size`.
    ///
    /// The page and total count are fetched inside a single transaction so that the `total`
    /// is consistent with the rows returned (no tearing if another writer commits in between).
    ///
    /// At steady state this table holds roughly one row per concurrently-testing agent, so
    /// the ordinary page-size cap from [`Pagination`] is more than enough headroom.
    pub(crate) async fn get_testruns_in_progress_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(Vec<TestRunInProgress>, i64)> {
        let mut tx = self.connection_pool.begin().await?;

        let rows = sqlx::query_as::<_, TestRunInProgress>(
            "SELECT * FROM testrun_in_progress ORDER BY started_at ASC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *tx)
        .await?;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM testrun_in_progress")
            .fetch_one(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok((rows, total))
    }

    /// Deletes all `testrun` rows whose `test_timestamp` is older than `cutoff`.
    ///
    /// Intended to be called periodically with `now - eviction_age` as the cutoff to keep
    /// the local database from growing unboundedly. Rows that are evicted are assumed to
    /// have already been submitted to the nym-api for persistent storage.
    ///
    /// Each run's measurement rows go with it (`ON DELETE CASCADE`), and any
    /// `node_test_state.last_testrun_id` that pointed at an evicted row is set to `NULL`. The
    /// kind's `last_tested_at` is deliberately left alone, so an evicted result does not make
    /// the node read as never-tested and jump the assignment queue.
    pub(crate) async fn evict_old_testruns(&self, cutoff: OffsetDateTime) -> anyhow::Result<u64> {
        let res = sqlx::query!("DELETE FROM testrun WHERE test_timestamp < ?", cutoff)
            .execute(&self.connection_pool)
            .await?;
        Ok(res.rows_affected())
    }

    /// Returns the id of the most recent run of `test_kind` that has been successfully submitted to
    /// the nym-api, or `None` if that stream has never submitted a batch.
    ///
    /// The watermark is per kind because each kind's ids come from its own results table, so an id
    /// means nothing outside its kind.
    pub(crate) async fn get_last_submitted_testrun_id(
        &self,
        test_kind: TestKind,
    ) -> anyhow::Result<Option<i64>> {
        let id = sqlx::query_scalar!(
            "SELECT last_submitted_testrun_id FROM submission_watermark WHERE test_kind = ?",
            test_kind
        )
        .fetch_optional(&self.connection_pool)
        .await?;
        Ok(id)
    }

    /// Records that every run of `test_kind` with `id <= testrun_id` has been successfully
    /// submitted to the nym-api, creating that stream's watermark row if this is its first batch.
    pub(crate) async fn set_last_submitted_testrun_id(
        &self,
        test_kind: TestKind,
        testrun_id: i64,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO submission_watermark (test_kind, last_submitted_testrun_id) VALUES (?, ?)
            ON CONFLICT (test_kind) DO UPDATE SET last_submitted_testrun_id = excluded.last_submitted_testrun_id
            "#,
            test_kind,
            testrun_id,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Fetches every run in `test_kind`'s results table with an id strictly greater than
    /// `after_id`, ordered by id ascending so the caller can pick the highest-id submitted row
    /// deterministically.
    ///
    /// `after_id = 0` (the default used before any batch has been submitted) returns every row of
    /// that kind, since each table's `id` is `AUTOINCREMENT` and therefore always `>= 1`.
    pub(crate) async fn get_testruns_after(
        &self,
        test_kind: TestKind,
        after_id: i64,
    ) -> anyhow::Result<Vec<CompletedTestRun>> {
        let mut conn = self.connection_pool.acquire().await?;
        match test_kind {
            TestKind::MixnodeLiveness => {
                get_mixnode_liveness_testruns_after(&mut conn, after_id).await
            }
            TestKind::GatewayLiveness => {
                get_gateway_liveness_testruns_after(&mut conn, after_id).await
            }
            TestKind::MixnodeStress => get_mixnode_stress_testruns_after(&mut conn, after_id).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::{
        ExercisedInterface, NewNymNode, NewTestRun, NodeTestState, NodeType, minimal_measurement,
        minimal_test_run, node_with_ips,
    };
    use std::net::IpAddr;
    use time::macros::datetime;

    async fn setup() -> StorageManager {
        crate::storage::NetworkMonitorStorage::in_memory()
            .await
            .storage_manager
    }

    fn node(id: i64, identity_key: &str) -> NewNymNode {
        node_with_ips(id, identity_key, "1.2.3.4")
    }

    /// A gateway-capable node, optionally announcing the client websocket port that the gateway
    /// liveness probe cannot open a session without.
    fn gateway_node(id: i64, clients_ws_port: Option<i64>) -> NewNymNode {
        NewNymNode {
            node_type: NodeType::Gateway,
            clients_ws_port,
            ..node(id, &format!("key_{id}"))
        }
    }

    /// A stress run with the single `mix_forwarding` measurement such a run always produces.
    fn mixnode_run(node_id: i64) -> (NewTestRun, Vec<TestRunMeasurement>) {
        (
            minimal_test_run(node_id),
            vec![minimal_measurement(ExercisedInterface::MixForwarding)],
        )
    }

    /// Inserts a run and returns its id, discarding the in-flight bookkeeping.
    async fn insert_run(db: &StorageManager, run: &NewTestRun) -> i64 {
        let measurements = vec![minimal_measurement(ExercisedInterface::MixForwarding)];
        db.insert_test_run(run, &measurements).await.unwrap().id
    }

    /// Seeds a single nym_node row so that testruns referencing `node_id` satisfy the FK.
    async fn seed_node(db: &StorageManager, node_id: i64) {
        db.batch_insert_or_update_nym_nodes(&[node(node_id, &format!("key_{node_id}"))])
            .await
            .unwrap();
    }

    /// Reads one pairing's work-state row, or `None` if neither the assignment nor a result has
    /// touched it yet. Goes through the model rather than `query!` because sqlx cannot infer a
    /// Rust type for a nullable `TIMESTAMP WITHOUT TIME ZONE` column.
    async fn work_state(
        db: &StorageManager,
        node_id: i64,
        test_kind: TestKind,
        tested_role: TestedRole,
    ) -> Option<NodeTestState> {
        sqlx::query_as::<_, NodeTestState>(
            "SELECT * FROM node_test_state WHERE node_id = ? AND test_kind = ? AND tested_role = ?",
        )
        .bind(node_id)
        .bind(test_kind)
        .bind(tested_role)
        .fetch_optional(&db.connection_pool)
        .await
        .unwrap()
    }

    /// Every work-state row a node holds, ordered by kind so assertions can index them.
    async fn work_states(db: &StorageManager, node_id: i64) -> Vec<NodeTestState> {
        sqlx::query_as::<_, NodeTestState>(
            "SELECT * FROM node_test_state WHERE node_id = ? ORDER BY test_kind",
        )
        .bind(node_id)
        .fetch_all(&db.connection_pool)
        .await
        .unwrap()
    }

    // A far-future cutoff that effectively disables the staleness gate,
    // used in tests that are not concerned with that behaviour.
    fn no_staleness_gate() -> OffsetDateTime {
        datetime!(9999-12-31 23:59:59 UTC)
    }

    /// A request for `pairing` at `now`, with an hour-long lease and the given staleness gate.
    fn request(
        pairing: TestPairing,
        now: OffsetDateTime,
        last_tested_before: OffsetDateTime,
        wave_size: usize,
    ) -> AssignmentRequest {
        AssignmentRequest {
            pairing,
            now,
            last_tested_before,
            expires_at: now + time::Duration::hours(1),
            wave_size,
        }
    }

    /// A `(stress, mixnode)` request, i.e. the one-target wave that pairing always asks for.
    fn stress_request(
        now: OffsetDateTime,
        last_tested_before: OffsetDateTime,
    ) -> AssignmentRequest {
        request(TestPairing::STRESS_MIXNODE, now, last_tested_before, 1)
    }

    /// Assigns the stress pairing at `now`, returning the single target such a request can produce.
    async fn assign(
        db: &StorageManager,
        now: OffsetDateTime,
        last_tested_before: OffsetDateTime,
    ) -> Option<AssignedTestrun> {
        db.assign_next_testruns(&stress_request(now, last_tested_before))
            .await
            .unwrap()
            .into_iter()
            .next()
    }

    /// Seeds a pairing's rotation pointer, standing in for an assignment of a (kind, role) the
    /// orchestrator cannot dispatch yet.
    async fn seed_rotation_pointer(
        db: &StorageManager,
        node_id: i64,
        test_kind: TestKind,
        tested_role: TestedRole,
        last_tested_ip: &str,
    ) {
        sqlx::query(
            "INSERT INTO node_test_state (node_id, test_kind, tested_role, last_tested_ip)
             VALUES (?, ?, ?, ?)",
        )
        .bind(node_id)
        .bind(test_kind)
        .bind(tested_role)
        .bind(last_tested_ip)
        .execute(&db.connection_pool)
        .await
        .unwrap();
    }

    /// Marks a node as in-progress with the stress/mixnode pairing and an hour-long lease.
    async fn mark_in_progress(db: &StorageManager, node_id: i64, started_at: OffsetDateTime) {
        db.mark_testrun_in_progress(
            node_id,
            started_at,
            started_at + time::Duration::hours(1),
            TestKind::Stress,
            TestedRole::Mixnode,
        )
        .await
        .unwrap()
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
            updated.clients_ws_port = Some(9000);

            let nodes = vec![updated, node(2, "key_b")];
            db.batch_insert_or_update_nym_nodes(&nodes).await.unwrap();

            let row = sqlx::query!(
                "SELECT mixnet_socket_address, noise_key, clients_ws_port FROM nym_node WHERE node_id = 1"
            )
            .fetch_one(&db.connection_pool)
            .await
            .unwrap();
            assert_eq!(row.mixnet_socket_address.as_deref(), Some("9.9.9.9:1789"));
            assert_eq!(row.noise_key.as_deref(), Some("new_noise"));
            assert_eq!(row.clients_ws_port, Some(9000));

            let count = sqlx::query_scalar!("SELECT COUNT(*) FROM nym_node")
                .fetch_one(&db.connection_pool)
                .await
                .unwrap();
            assert_eq!(count, 2);
        }

        /// The bond-only write, i.e. what a node whose describe failed this cycle gets.
        async fn touch(db: &StorageManager, node_id: i64, last_seen_bonded: OffsetDateTime) {
            db.batch_touch_bonded_nodes(&[BondedNymNode {
                node_id,
                identity_key: format!("key_{node_id}"),
                last_seen_bonded,
            }])
            .await
            .unwrap()
        }

        // A failed describe must not cost the node everything an earlier cycle learned: nulling
        // these columns fails every eligibility predicate at once, which would drop a node that is
        // merely slow out of EVERY kind until a later cycle answered.
        #[tokio::test]
        async fn a_bond_only_refresh_keeps_what_was_already_learned() {
            let db = setup().await;
            let mut described = node(1, "key_1");
            described.clients_ws_port = Some(9000);
            described.node_type = NodeType::MixnodeAndGateway;
            db.batch_insert_or_update_nym_nodes(&[described])
                .await
                .unwrap();

            touch(&db, 1, datetime!(2025-06-02 00:00:00 UTC)).await;

            let node = db.get_nym_node_by_id(1).await.unwrap().unwrap().inner;
            assert_eq!(node.mixnet_socket_address.as_deref(), Some("1.2.3.4:1789"));
            assert_eq!(node.noise_key.as_deref(), Some("placeholder_noise_key"));
            assert_eq!(node.sphinx_key.as_deref(), Some("placeholder_sphinx_key"));
            assert_eq!(node.announced_ips.as_deref(), Some("1.2.3.4"));
            assert_eq!(node.key_rotation_id, Some(0));
            assert_eq!(node.clients_ws_port, Some(9000));
            assert_eq!(node.node_type, NodeType::MixnodeAndGateway);

            // the one thing it does record is that the bond is still there
            assert_eq!(node.last_seen_bonded, datetime!(2025-06-02 00:00:00 UTC));
        }

        // A node seen for the first time still has to exist as a row, and its empty describe columns
        // are the one state that genuinely means "never described" rather than "described once".
        #[tokio::test]
        async fn a_bond_only_refresh_inserts_an_undescribed_node() {
            let db = setup().await;
            touch(&db, 7, datetime!(2025-06-02 00:00:00 UTC)).await;

            let node = db.get_nym_node_by_id(7).await.unwrap().unwrap().inner;
            assert_eq!(node.identity_key, "key_7");
            assert!(node.mixnet_socket_address.is_none());
            assert!(node.noise_key.is_none());
            assert!(node.sphinx_key.is_none());
            assert!(node.clients_ws_port.is_none());
            assert_eq!(node.node_type, NodeType::Unknown);
        }

        // and being described afterwards fills it in, so a node that answers late is not stuck as a
        // stub
        #[tokio::test]
        async fn a_later_describe_replaces_a_bond_only_row() {
            let db = setup().await;
            touch(&db, 1, datetime!(2025-06-02 00:00:00 UTC)).await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_1")])
                .await
                .unwrap();

            let node = db.get_nym_node_by_id(1).await.unwrap().unwrap().inner;
            assert_eq!(node.noise_key.as_deref(), Some("placeholder_noise_key"));
            assert_eq!(node.node_type, NodeType::Mixnode);
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
            seed_node(&db, 1).await;
            let id1 = insert_run(&db, &minimal_test_run(1)).await;
            let id2 = insert_run(&db, &minimal_test_run(1)).await;
            assert!(id2 > id1);
        }

        #[tokio::test]
        async fn persists_run_level_fields() {
            let db = setup().await;
            seed_node(&db, 1).await;
            let mut run = minimal_test_run(1);
            run.time_taken_us = 1234;
            run.error = Some("timeout".to_string());
            let id = insert_run(&db, &run).await;

            let row = sqlx::query!(
                "SELECT test_kind, tested_role, tested_address, time_taken_us, error
                 FROM testrun WHERE id = ?",
                id
            )
            .fetch_one(&db.connection_pool)
            .await
            .unwrap();
            assert_eq!(row.test_kind, "stress");
            assert_eq!(row.tested_role, "mixnode");
            assert_eq!(row.tested_address, "1.2.3.4:1789");
            assert_eq!(row.time_taken_us, 1234);
            assert_eq!(row.error.as_deref(), Some("timeout"));
        }

        // a gateway liveness run carries one measurement per phase, and both have to land under the
        // same run so that a healthy ingest with a dead delivery stays visible
        #[tokio::test]
        async fn persists_every_measurement_of_a_run() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let mut run = minimal_test_run(1);
            run.test_kind = TestKind::Liveness;
            run.tested_role = TestedRole::Gateway;

            let mut ingest = minimal_measurement(ExercisedInterface::ClientIngest);
            ingest.packets_sent = 100;
            ingest.packets_received = 99;
            let mut delivery = minimal_measurement(ExercisedInterface::ClientDelivery);
            delivery.packets_sent = 100;
            delivery.packets_received = 0;

            let id = db
                .insert_test_run(&run, &[ingest, delivery])
                .await
                .unwrap()
                .id;

            let stored = db.get_testrun_by_id(id).await.unwrap().unwrap();
            assert_eq!(stored.measurements.len(), 2);
            assert_eq!(
                stored
                    .measurement(ExercisedInterface::ClientIngest)
                    .unwrap()
                    .packets_received,
                99
            );
            assert_eq!(
                stored
                    .measurement(ExercisedInterface::ClientDelivery)
                    .unwrap()
                    .packets_received,
                0
            );
        }

        #[tokio::test]
        async fn records_the_pairings_work_state() {
            let db = setup().await;
            seed_node(&db, 1).await;
            let (run, measurements) = mixnode_run(1);
            let id = db.insert_test_run(&run, &measurements).await.unwrap().id;

            let state = work_state(&db, 1, TestKind::Stress, TestedRole::Mixnode)
                .await
                .unwrap();
            assert_eq!(state.last_testrun_id, Some(id));
            assert_eq!(state.last_tested_at, Some(run.test_timestamp));
        }

        // each (kind, role) pairing keeps its own staleness position, so recording a run under one
        // must not touch another's
        #[tokio::test]
        async fn one_pairings_result_leaves_the_others_untouched() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let (stress, measurements) = mixnode_run(1);
            db.insert_test_run(&stress, &measurements).await.unwrap();

            assert_eq!(work_states(&db, 1).await.len(), 1);

            let mut liveness = minimal_test_run(1);
            liveness.test_kind = TestKind::Liveness;
            liveness.test_timestamp = datetime!(2025-06-02 12:00:00 UTC);
            db.insert_test_run(&liveness, &measurements).await.unwrap();

            let states = work_states(&db, 1).await;
            assert_eq!(states.len(), 2);
            // each pairing carries the timestamp of its own run, not of the other's
            assert_eq!(states[0].test_kind, TestKind::Liveness);
            assert_eq!(states[0].last_tested_at, Some(liveness.test_timestamp));
            assert_eq!(states[1].test_kind, TestKind::Stress);
            assert_eq!(states[1].last_tested_at, Some(stress.test_timestamp));
        }

        #[tokio::test]
        async fn releases_the_nodes_in_flight_lock() {
            let db = setup().await;
            seed_node(&db, 1).await;
            mark_in_progress(&db, 1, datetime!(2025-06-01 11:00:00 UTC)).await;

            let (run, measurements) = mixnode_run(1);
            let inserted = db.insert_test_run(&run, &measurements).await.unwrap();
            assert_eq!(inserted.cleared_in_progress, 1);

            let count = sqlx::query_scalar!("SELECT COUNT(*) FROM testrun_in_progress")
                .fetch_one(&db.connection_pool)
                .await
                .unwrap();
            assert_eq!(count, 0);
        }

        // the insert does not require a lock to exist: the submission path rejects a result whose
        // lease expired, but the sweep can still reap the row in the window between that check and
        // this insert, and the reported count is what keeps the in-flight gauge from drifting
        #[tokio::test]
        async fn releasing_an_already_reaped_lock_reports_nothing_cleared() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let (run, measurements) = mixnode_run(1);
            let inserted = db.insert_test_run(&run, &measurements).await.unwrap();
            assert_eq!(inserted.cleared_in_progress, 0);
            assert!(db.get_testrun_by_id(inserted.id).await.unwrap().is_some());
        }
    }

    mod get_testrun_in_progress {
        use super::*;

        #[tokio::test]
        async fn returns_the_dispatched_kind_and_role() {
            let db = setup().await;
            seed_node(&db, 1).await;
            db.mark_testrun_in_progress(
                1,
                datetime!(2025-06-01 11:00:00 UTC),
                datetime!(2025-06-01 11:05:00 UTC),
                TestKind::Liveness,
                TestedRole::Gateway,
            )
            .await
            .unwrap();

            let row = db.get_testrun_in_progress(1).await.unwrap().unwrap();
            assert_eq!(row.test_kind, TestKind::Liveness);
            assert_eq!(row.tested_role, TestedRole::Gateway);
            assert_eq!(row.expires_at, datetime!(2025-06-01 11:05:00 UTC));
        }

        #[tokio::test]
        async fn returns_none_for_a_node_with_no_open_run() {
            let db = setup().await;
            seed_node(&db, 1).await;
            assert!(db.get_testrun_in_progress(1).await.unwrap().is_none());
        }
    }

    mod mark_testrun_in_progress {
        use super::*;

        #[tokio::test]
        async fn inserts_row() {
            let db = setup().await;
            seed_node(&db, 1).await;
            mark_in_progress(&db, 1, datetime!(2025-06-01 10:00:00 UTC)).await;

            let count =
                sqlx::query_scalar!("SELECT COUNT(*) FROM testrun_in_progress WHERE node_id = 1")
                    .fetch_one(&db.connection_pool)
                    .await
                    .unwrap();
            assert_eq!(count, 1);
        }

        // one test at a time per node, across kinds AND roles: the key is the node alone
        #[tokio::test]
        async fn rejects_duplicate() {
            let db = setup().await;
            seed_node(&db, 1).await;
            mark_in_progress(&db, 1, datetime!(2025-06-01 10:00:00 UTC)).await;

            let result = db
                .mark_testrun_in_progress(
                    1,
                    datetime!(2025-06-01 11:00:00 UTC),
                    datetime!(2025-06-01 11:05:00 UTC),
                    TestKind::Liveness,
                    TestedRole::Gateway,
                )
                .await;
            assert!(result.is_err());
        }
    }

    mod clear_expired_testruns_in_progress {
        use super::*;

        /// Marks a node in progress with an explicit lease deadline.
        async fn lease(
            db: &StorageManager,
            node_id: i64,
            started_at: OffsetDateTime,
            expires_at: OffsetDateTime,
        ) {
            lease_of(db, node_id, TestKind::Stress, started_at, expires_at).await
        }

        /// The same, under a chosen kind. The role plays no part in expiry, so it stays fixed.
        async fn lease_of(
            db: &StorageManager,
            node_id: i64,
            test_kind: TestKind,
            started_at: OffsetDateTime,
            expires_at: OffsetDateTime,
        ) {
            db.mark_testrun_in_progress(
                node_id,
                started_at,
                expires_at,
                test_kind,
                TestedRole::Mixnode,
            )
            .await
            .unwrap()
        }

        async fn remaining(db: &StorageManager) -> Vec<i64> {
            sqlx::query_scalar!("SELECT node_id FROM testrun_in_progress ORDER BY node_id")
                .fetch_all(&db.connection_pool)
                .await
                .unwrap()
        }

        // each row is judged by its OWN deadline. node 2 is the case a cutoff derived from one
        // global timeout got wrong: dispatched longest ago, but under a lease that is still running,
        // so reaping it would hand its node to a second agent while the first was still working
        #[tokio::test]
        async fn removes_only_rows_whose_lease_has_run_out() {
            let db = setup().await;
            for node_id in 1..=4 {
                seed_node(&db, node_id).await;
            }
            let now = datetime!(2025-06-01 12:00:00 UTC);

            // dispatched two hours ago on a five-minute lease
            lease(
                &db,
                1,
                datetime!(2025-06-01 10:00:00 UTC),
                datetime!(2025-06-01 10:05:00 UTC),
            )
            .await;
            // dispatched four hours ago, but leased until this evening
            lease(
                &db,
                2,
                datetime!(2025-06-01 08:00:00 UTC),
                datetime!(2025-06-01 20:00:00 UTC),
            )
            .await;
            // dispatched a minute ago, lease still running
            lease(
                &db,
                3,
                datetime!(2025-06-01 11:59:00 UTC),
                datetime!(2025-06-01 12:04:00 UTC),
            )
            .await;
            // expiring exactly now: the comparison is strict, so it survives this sweep
            lease(&db, 4, datetime!(2025-06-01 11:55:00 UTC), now).await;

            let cleared = db.clear_expired_testruns_in_progress(now).await.unwrap();
            assert_eq!(cleared.get(&TestKind::Stress).copied(), Some(1));
            assert_eq!(remaining(&db).await, vec![2, 3, 4]);
        }

        // the breakdown is the whole point of counting before the delete: it says WHICH kind's lease
        // is too short for the work it covers, which a single total cannot, since the two kinds run
        // on leases minutes apart and so expire at different rates even when both are healthy
        #[tokio::test]
        async fn expiries_are_counted_per_kind() {
            let db = setup().await;
            for node_id in 1..=3 {
                seed_node(&db, node_id).await;
            }
            let expired_at = datetime!(2025-06-01 11:00:00 UTC);

            lease_of(&db, 1, TestKind::Stress, expired_at, expired_at).await;
            lease_of(&db, 2, TestKind::Liveness, expired_at, expired_at).await;
            lease_of(&db, 3, TestKind::Liveness, expired_at, expired_at).await;

            let cleared = db
                .clear_expired_testruns_in_progress(datetime!(2025-06-01 12:00:00 UTC))
                .await
                .unwrap();

            assert_eq!(cleared.get(&TestKind::Stress).copied(), Some(1));
            assert_eq!(cleared.get(&TestKind::Liveness).copied(), Some(2));
            assert!(remaining(&db).await.is_empty());
        }

        #[tokio::test]
        async fn clears_nothing_when_every_lease_is_live() {
            let db = setup().await;
            seed_node(&db, 1).await;
            lease(
                &db,
                1,
                datetime!(2025-06-01 11:00:00 UTC),
                datetime!(2025-06-01 13:00:00 UTC),
            )
            .await;

            let cleared = db
                .clear_expired_testruns_in_progress(datetime!(2025-06-01 12:00:00 UTC))
                .await
                .unwrap();
            assert!(cleared.is_empty());
            assert_eq!(remaining(&db).await, vec![1]);
        }

        // the per-kind gauges are published from this count, and a kind absent from the map is
        // published as zero - so absence has to mean "none in flight", not "not measured"
        #[tokio::test]
        async fn in_flight_rows_are_counted_per_kind_and_a_drained_kind_is_absent() {
            let db = setup().await;
            for node_id in 1..=3 {
                seed_node(&db, node_id).await;
            }
            let started_at = datetime!(2025-06-01 12:00:00 UTC);
            let expires_at = datetime!(2025-06-01 13:00:00 UTC);

            lease_of(&db, 1, TestKind::Liveness, started_at, expires_at).await;
            lease_of(&db, 2, TestKind::Liveness, started_at, expires_at).await;

            let counts = db.count_testruns_in_progress_by_kind().await.unwrap();
            assert_eq!(counts.get(&TestKind::Liveness).copied(), Some(2));
            assert_eq!(counts.get(&TestKind::Stress), None);
        }
    }

    mod evict_old_testruns {
        use super::*;

        #[tokio::test]
        async fn evicts_runs_older_than_cutoff() {
            let db = setup().await;
            seed_node(&db, 1).await;
            let mut old_run = minimal_test_run(1);
            old_run.test_timestamp = datetime!(2025-01-01 00:00:00 UTC);
            let old_id = insert_run(&db, &old_run).await;

            let mut recent_run = minimal_test_run(1);
            recent_run.test_timestamp = datetime!(2025-06-01 12:00:00 UTC);
            let recent_id = insert_run(&db, &recent_run).await;

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
            seed_node(&db, 1).await;
            let mut run = minimal_test_run(1);
            run.test_timestamp = datetime!(2025-03-01 00:00:00 UTC);
            let id = insert_run(&db, &run).await;

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

        // measurements are children of their run, so they must not outlive it - a leaked row would
        // also collide with the (testrun_id, interface) key if that id were ever reissued
        #[tokio::test]
        async fn takes_each_evicted_runs_measurements_with_it() {
            let db = setup().await;
            seed_node(&db, 1).await;

            // a gateway-shaped run, so the eviction has two child rows to remove rather than one
            let mut old_run = minimal_test_run(1);
            old_run.test_kind = TestKind::Liveness;
            old_run.tested_role = TestedRole::Gateway;
            old_run.test_timestamp = datetime!(2025-01-01 00:00:00 UTC);
            db.insert_test_run(
                &old_run,
                &[
                    minimal_measurement(ExercisedInterface::ClientIngest),
                    minimal_measurement(ExercisedInterface::ClientDelivery),
                ],
            )
            .await
            .unwrap();

            let mut recent_run = minimal_test_run(1);
            recent_run.test_timestamp = datetime!(2025-06-01 12:00:00 UTC);
            let recent_id = insert_run(&db, &recent_run).await;

            let count = sqlx::query_scalar!("SELECT COUNT(*) FROM testrun_measurement")
                .fetch_one(&db.connection_pool)
                .await
                .unwrap();
            assert_eq!(count, 3);

            db.evict_old_testruns(datetime!(2025-03-01 00:00:00 UTC))
                .await
                .unwrap();

            // only the surviving run's single measurement is left, and it is still attached to it
            let surviving: Vec<i64> =
                sqlx::query_scalar!("SELECT testrun_id FROM testrun_measurement")
                    .fetch_all(&db.connection_pool)
                    .await
                    .unwrap();
            assert_eq!(surviving, vec![recent_id]);
        }

        #[tokio::test]
        async fn does_nothing_when_no_old_runs() {
            let db = setup().await;
            seed_node(&db, 1).await;
            insert_run(&db, &minimal_test_run(1)).await;

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

    mod assign_next_testruns {
        use super::*;

        // The role gate is the branch of the composed query that no stress request reaches, so this
        // exercises it against one eligible node and one decoy per predicate it applies.
        // One assignment, many targets, each locked and rotated in its own right - the property that
        // separates a wave from the single-target assignment this used to be.
        #[tokio::test]
        async fn a_wave_locks_every_target_it_returns_and_stops_at_the_wave_size() {
            let db = setup().await;
            for node_id in 1..=3 {
                seed_node(&db, node_id).await;
            }

            let now = datetime!(2025-06-01 12:00:00 UTC);
            let wave = db
                .assign_next_testruns(&AssignmentRequest {
                    expires_at: now + time::Duration::minutes(1),
                    ..request(TestPairing::LIVENESS_MIXNODE, now, no_staleness_gate(), 2)
                })
                .await
                .unwrap();

            assert_eq!(wave.len(), 2);

            for target in &wave {
                let node_id = target.node.inner.node_id;

                // a lock per target, each carrying this wave's lease rather than one shared row
                let row = db.get_testrun_in_progress(node_id).await.unwrap().unwrap();
                assert_eq!(row.expires_at, now + time::Duration::minutes(1));
                assert_eq!(row.test_kind, TestKind::Liveness);
                assert_eq!(row.tested_role, TestedRole::Mixnode);

                // and a rotation pointer per target, under the pairing that was dispatched
                let state = work_state(&db, node_id, TestKind::Liveness, TestedRole::Mixnode)
                    .await
                    .unwrap();
                assert_eq!(state.last_tested_ip.as_deref(), Some("1.2.3.4"));
            }

            // the target the cap left behind is still assignable, i.e. it was passed over rather
            // than locked
            let remaining = db
                .assign_next_testruns(&request(
                    TestPairing::LIVENESS_MIXNODE,
                    now,
                    no_staleness_gate(),
                    2,
                ))
                .await
                .unwrap();
            assert_eq!(remaining.len(), 1);
        }

        #[tokio::test]
        async fn a_gateway_request_takes_only_gateways_announcing_a_websocket_port() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[
                gateway_node(1, Some(9000)),
                // gateway-capable, but has never reported the port the session needs
                gateway_node(2, None),
                // announces a port, but is not gateway-capable
                NewNymNode {
                    clients_ws_port: Some(9000),
                    ..node(3, "key_3")
                },
            ])
            .await
            .unwrap();

            let now = datetime!(2025-06-01 12:00:00 UTC);
            let assigned = db
                .assign_next_testruns(&request(
                    TestPairing::LIVENESS_GATEWAY,
                    now,
                    no_staleness_gate(),
                    2,
                ))
                .await
                .unwrap();

            let assigned_ids: Vec<_> = assigned
                .iter()
                .map(|target| target.node.inner.node_id)
                .collect();
            assert_eq!(assigned_ids, vec![1]);

            // the lock records the pairing that was dispatched, not the one stress would have used
            let row = db.get_testrun_in_progress(1).await.unwrap().unwrap();
            assert_eq!(row.test_kind, TestKind::Liveness);
            assert_eq!(row.tested_role, TestedRole::Gateway);
        }

        #[tokio::test]
        async fn returns_none_when_no_nodes() {
            let db = setup().await;
            let result = assign(&db, datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate()).await;
            assert!(result.is_none());
        }

        #[tokio::test]
        async fn returns_none_when_all_nodes_in_progress() {
            let db = setup().await;
            seed_node(&db, 1).await;
            assign(&db, datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate()).await;

            let result = assign(&db, datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate()).await;
            assert!(result.is_none());
        }

        #[tokio::test]
        async fn inserts_in_progress_row_carrying_the_lease_kind_and_role() {
            let db = setup().await;
            seed_node(&db, 1).await;
            let assigned =
                assign(&db, datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate()).await;
            assert!(assigned.is_some());

            let row = db.get_testrun_in_progress(1).await.unwrap().unwrap();
            assert_eq!(row.started_at, datetime!(2025-06-01 12:00:00 UTC));
            assert_eq!(row.expires_at, datetime!(2025-06-01 13:00:00 UTC));
            assert_eq!(row.test_kind, TestKind::Stress);
            assert_eq!(row.tested_role, TestedRole::Mixnode);
        }

        #[tokio::test]
        async fn advances_the_pairings_rotation_pointer_on_handout() {
            let db = setup().await;
            seed_node(&db, 1).await;
            assign(&db, datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate())
                .await
                .unwrap();

            let state = work_state(&db, 1, TestKind::Stress, TestedRole::Mixnode)
                .await
                .unwrap();
            assert_eq!(state.last_tested_ip.as_deref(), Some("1.2.3.4"));
            // the assignment records only the pointer; staleness moves when a result arrives
            assert!(state.last_tested_at.is_none());
        }

        #[tokio::test]
        async fn prefers_never_tested_node_over_stale_one() {
            let db = setup().await;
            seed_node(&db, 1).await;
            seed_node(&db, 2).await;

            // give node 1 a completed test run
            insert_run(&db, &minimal_test_run(1)).await;

            // node 2 has never been tested — it should be picked first
            let assigned = assign(&db, datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate())
                .await
                .unwrap();
            assert_eq!(assigned.node.inner.node_id, 2);
        }

        #[tokio::test]
        async fn prefers_older_testrun_over_newer_one() {
            let db = setup().await;
            seed_node(&db, 1).await;
            seed_node(&db, 2).await;

            let mut old_run = minimal_test_run(1);
            old_run.test_timestamp = datetime!(2025-01-01 00:00:00 UTC);
            insert_run(&db, &old_run).await;

            let mut new_run = minimal_test_run(2);
            new_run.test_timestamp = datetime!(2025-06-01 12:00:00 UTC);
            insert_run(&db, &new_run).await;

            // node 1 has the older run — it should be picked
            let assigned = assign(&db, datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate())
                .await
                .unwrap();
            assert_eq!(assigned.node.inner.node_id, 1);
        }

        #[tokio::test]
        async fn skips_node_already_in_progress() {
            let db = setup().await;
            seed_node(&db, 1).await;
            seed_node(&db, 2).await;

            // both have no test run; node 1 is manually put in progress
            mark_in_progress(&db, 1, datetime!(2025-06-01 11:00:00 UTC)).await;

            let assigned = assign(&db, datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate())
                .await
                .unwrap();
            assert_eq!(assigned.node.inner.node_id, 2);
        }

        // the in-flight lock is not per kind: a node being stress-tested must not be handed out for
        // a liveness probe either, since concurrent measurement biases both
        #[tokio::test]
        async fn skips_node_held_by_another_kinds_run() {
            let db = setup().await;
            seed_node(&db, 1).await;
            db.mark_testrun_in_progress(
                1,
                datetime!(2025-06-01 11:00:00 UTC),
                datetime!(2025-06-01 11:05:00 UTC),
                TestKind::Liveness,
                TestedRole::Gateway,
            )
            .await
            .unwrap();

            let result = assign(&db, datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate()).await;
            assert!(result.is_none());
        }

        // and the other direction, which is the one the liveness kind depends on: a node being
        // stress-tested at high rate must not be measured by a liveness probe at the same time, or
        // both results describe something other than the node
        #[tokio::test]
        async fn a_stress_run_in_flight_blocks_a_liveness_assignment() {
            let db = setup().await;
            seed_node(&db, 1).await;
            let now = datetime!(2025-06-01 12:00:00 UTC);
            assert!(assign(&db, now, no_staleness_gate()).await.is_some());

            let wave = db
                .assign_next_testruns(&request(
                    TestPairing::LIVENESS_MIXNODE,
                    now,
                    no_staleness_gate(),
                    10,
                ))
                .await
                .unwrap();
            assert!(wave.is_empty());
        }

        // The per-node lock is the WHOLE of the exclusion between kinds: there is no cooldown after
        // it clears. The staleness gate here would reject the node if the stress run's timestamp
        // were consulted for liveness, so this also pins that each pairing reads only its own.
        #[tokio::test]
        async fn a_node_freed_by_one_kind_is_immediately_assignable_by_another() {
            let db = setup().await;
            seed_node(&db, 1).await;
            let now = datetime!(2025-06-01 12:00:00 UTC);

            // a stress run takes the node, completes, and releases the lock
            assert!(assign(&db, now, no_staleness_gate()).await.is_some());
            insert_run(
                &db,
                &NewTestRun {
                    test_timestamp: now,
                    ..minimal_test_run(1)
                },
            )
            .await;
            assert!(db.get_testrun_in_progress(1).await.unwrap().is_none());

            // at the very same instant, under a gate an hour in the past
            let wave = db
                .assign_next_testruns(&request(
                    TestPairing::LIVENESS_MIXNODE,
                    now,
                    now - time::Duration::hours(1),
                    10,
                ))
                .await
                .unwrap();

            assert_eq!(wave.len(), 1);
            assert_eq!(wave[0].node.inner.node_id, 1);
        }

        #[tokio::test]
        async fn skips_node_tested_too_recently() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let mut run = minimal_test_run(1);
            run.test_timestamp = datetime!(2025-06-01 12:00:00 UTC);
            insert_run(&db, &run).await;

            // cutoff is before the last test — node is not stale enough
            let result = assign(
                &db,
                datetime!(2025-06-01 13:00:00 UTC),
                datetime!(2025-06-01 11:00:00 UTC),
            )
            .await;
            assert!(result.is_none());
        }

        #[tokio::test]
        async fn returns_node_tested_sufficiently_long_ago() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let mut run = minimal_test_run(1);
            run.test_timestamp = datetime!(2025-06-01 12:00:00 UTC);
            insert_run(&db, &run).await;

            // cutoff is after the last test — node is eligible
            let assigned = assign(
                &db,
                datetime!(2025-06-01 14:00:00 UTC),
                datetime!(2025-06-01 13:00:00 UTC),
            )
            .await;
            assert!(assigned.is_some());
        }

        // a run recorded under a different pairing must not gate this one: staleness is per
        // (kind, role), which is what keeps a dual-role node eligible for both liveness probes
        #[tokio::test]
        async fn another_pairings_recent_run_does_not_gate_this_one() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let mut liveness = minimal_test_run(1);
            liveness.test_kind = TestKind::Liveness;
            liveness.test_timestamp = datetime!(2025-06-01 12:00:00 UTC);
            insert_run(&db, &liveness).await;

            let assigned = assign(
                &db,
                datetime!(2025-06-01 13:00:00 UTC),
                datetime!(2025-06-01 11:00:00 UTC),
            )
            .await;
            assert!(assigned.is_some());
        }

        #[tokio::test]
        async fn never_tested_node_bypasses_staleness_gate() {
            let db = setup().await;
            seed_node(&db, 1).await;
            seed_node(&db, 2).await;

            // node 1 was tested very recently
            let mut run = minimal_test_run(1);
            run.test_timestamp = datetime!(2025-06-01 12:00:00 UTC);
            insert_run(&db, &run).await;

            // cutoff is before node 1's last test — it is filtered out
            // node 2 has never been tested and must still be returned
            let assigned = assign(
                &db,
                datetime!(2025-06-01 13:00:00 UTC),
                datetime!(2025-06-01 11:00:00 UTC),
            )
            .await
            .unwrap();
            assert_eq!(assigned.node.inner.node_id, 2);
        }
    }

    /// The two properties the three-part `(node_id, test_kind, tested_role)` work-state key exists
    /// to provide, driven through the real assignment and eviction paths.
    mod peek_pairing_head {
        use super::*;

        /// Everything that makes a node ineligible, so the peek is asked about a population where
        /// only one node qualifies and every exclusion is represented.
        async fn seed_mixed_population(db: &StorageManager) {
            db.batch_insert_or_update_nym_nodes(&[
                // eligible, never tested by any pairing
                node(1, "key_1"),
                // eligible on paper, but locked by a run of another kind entirely
                node(2, "key_2"),
                // untestable: a gateway cannot be probed as a mixing hop
                NewNymNode {
                    node_type: NodeType::Gateway,
                    ..node(3, "key_3")
                },
                // untestable: never answered its self-described endpoint
                NewNymNode {
                    noise_key: None,
                    ..node(4, "key_4")
                },
            ])
            .await
            .unwrap();
            mark_in_progress(db, 2, datetime!(2025-06-01 11:00:00 UTC)).await;
        }

        #[tokio::test]
        async fn a_pairing_with_nothing_eligible_has_no_head() {
            let db = setup().await;
            let head = db
                .peek_pairing_head(TestPairing::LIVENESS_MIXNODE, no_staleness_gate())
                .await
                .unwrap();
            assert!(head.is_none());
        }

        #[tokio::test]
        async fn a_never_tested_node_reads_as_the_most_overdue_head() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let head = db
                .peek_pairing_head(TestPairing::LIVENESS_MIXNODE, no_staleness_gate())
                .await
                .unwrap();
            assert_eq!(head, Some(PairingHead::NeverTested));

            // and it outranks any measured node, which is what makes the most overdue head the
            // minimum of the pairings a kind holds
            assert!(
                PairingHead::NeverTested
                    < PairingHead::LastTestedAt(datetime!(1970-01-01 00:00:00 UTC))
            );
        }

        #[tokio::test]
        async fn a_measured_node_reads_as_its_last_run() {
            let db = setup().await;
            seed_node(&db, 1).await;
            insert_run(
                &db,
                &NewTestRun {
                    test_kind: TestKind::Liveness,
                    test_timestamp: datetime!(2025-06-01 09:00:00 UTC),
                    ..minimal_test_run(1)
                },
            )
            .await;

            let head = db
                .peek_pairing_head(TestPairing::LIVENESS_MIXNODE, no_staleness_gate())
                .await
                .unwrap();
            assert_eq!(
                head,
                Some(PairingHead::LastTestedAt(datetime!(
                    2025-06-01 09:00:00 UTC
                )))
            );

            // the same node under a pairing that has never measured it still reads as never tested,
            // so one pairing's progress cannot answer for another's
            let other = db
                .peek_pairing_head(TestPairing::STRESS_MIXNODE, no_staleness_gate())
                .await
                .unwrap();
            assert_eq!(other, Some(PairingHead::NeverTested));
        }

        // The peek and the assignment are separate hand-written queries, each keeping its binds
        // beside its own placeholders. That independence is only safe while they agree on who is
        // eligible and in what order, so this asserts the peek describes the very row the
        // assignment then takes, against a population exercising every exclusion.
        #[tokio::test]
        async fn the_peek_describes_the_node_the_assignment_takes() {
            let db = setup().await;
            seed_mixed_population(&db).await;

            let now = datetime!(2025-06-01 12:00:00 UTC);
            let head = db
                .peek_pairing_head(TestPairing::LIVENESS_MIXNODE, no_staleness_gate())
                .await
                .unwrap();

            let assigned = db
                .assign_next_testruns(&request(
                    TestPairing::LIVENESS_MIXNODE,
                    now,
                    no_staleness_gate(),
                    1,
                ))
                .await
                .unwrap();

            // the peek said there was work, and the assignment took exactly the node it described
            assert_eq!(head, Some(PairingHead::NeverTested));
            assert_eq!(assigned.len(), 1);
            assert_eq!(assigned[0].node.inner.node_id, 1);

            // with that node locked, both queries agree the pairing is drained
            let head = db
                .peek_pairing_head(TestPairing::LIVENESS_MIXNODE, no_staleness_gate())
                .await
                .unwrap();
            let assigned = db
                .assign_next_testruns(&request(
                    TestPairing::LIVENESS_MIXNODE,
                    now,
                    no_staleness_gate(),
                    1,
                ))
                .await
                .unwrap();
            assert!(head.is_none());
            assert!(assigned.is_empty());
        }
    }

    mod per_pairing_work_state {
        use super::*;

        // the rotation pointer is per pairing, so one pairing walking the node's address set must
        // leave another's position where it was, and must not continue from it either.
        //
        // the two decoy pairings differ from the assigned one in ONE component each - one in the
        // role, one in the kind - so that each half of the key is pinned separately. a decoy
        // differing in both would be excluded by either predicate alone, and the test would still
        // pass with one of them dropped. (stress, gateway) is not a pairing the orchestrator
        // dispatches; it is here precisely because the role is then the only thing distinguishing
        // it. the second pairing is seeded rather than assigned because only (stress, mixnode) is
        // dispatchable until per-kind selection lands
        #[tokio::test]
        async fn one_pairing_walking_the_address_set_leaves_anothers_pointer_alone() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node_with_ips(1, "key_a", "1.2.3.4,5.6.7.8")])
                .await
                .unwrap();

            // same kind, different role
            seed_rotation_pointer(&db, 1, TestKind::Stress, TestedRole::Gateway, "1.2.3.4").await;
            // same role, different kind
            seed_rotation_pointer(&db, 1, TestKind::Liveness, TestedRole::Mixnode, "1.2.3.4").await;

            // the assigned pairing has no pointer of its own yet, so it starts at the beginning of
            // the set rather than continuing from where either decoy had got to
            let first = assign(&db, datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate())
                .await
                .unwrap();
            assert_eq!(first.tested_ip, "1.2.3.4".parse::<IpAddr>().unwrap());

            // record the run, which releases the node's lock and moves its staleness position
            insert_run(&db, &minimal_test_run(1)).await;

            let second = assign(
                &db,
                datetime!(2025-06-01 13:00:00 UTC),
                datetime!(2025-06-01 12:30:00 UTC),
            )
            .await
            .unwrap();
            assert_eq!(second.tested_ip, "5.6.7.8".parse::<IpAddr>().unwrap());

            let assigned = work_state(&db, 1, TestKind::Stress, TestedRole::Mixnode)
                .await
                .unwrap();
            assert_eq!(assigned.last_tested_ip.as_deref(), Some("5.6.7.8"));

            // neither decoy moved, so the upsert wrote only its own pairing's row
            for (test_kind, tested_role) in [
                (TestKind::Stress, TestedRole::Gateway),
                (TestKind::Liveness, TestedRole::Mixnode),
            ] {
                let decoy = work_state(&db, 1, test_kind, tested_role).await.unwrap();
                assert_eq!(decoy.last_tested_ip.as_deref(), Some("1.2.3.4"));
            }
        }

        // the defect that denormalising `last_tested_at` fixes: read through a join onto the last
        // run, an evicted result made the node read as never-tested, so it jumped the assignment
        // queue ahead of nodes that genuinely had not been measured
        #[tokio::test]
        async fn evicting_a_result_leaves_the_pairings_staleness_position_intact() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let run = minimal_test_run(1);
            let run_id = insert_run(&db, &run).await;

            db.evict_old_testruns(datetime!(2025-06-02 00:00:00 UTC))
                .await
                .unwrap();
            assert!(db.get_testrun_by_id(run_id).await.unwrap().is_none());

            let state = work_state(&db, 1, TestKind::Stress, TestedRole::Mixnode)
                .await
                .unwrap();
            // the pointer to the run goes with the run itself...
            assert!(state.last_testrun_id.is_none());
            // ...while the staleness position it established survives, which is the whole reason
            // that timestamp is stored rather than joined
            assert_eq!(state.last_tested_at, Some(run.test_timestamp));

            // and behaviourally: the node is still gated, rather than jumping the queue
            let assigned = assign(
                &db,
                datetime!(2025-06-01 12:30:00 UTC),
                datetime!(2025-06-01 11:00:00 UTC),
            )
            .await;
            assert!(assigned.is_none());
        }
    }

    mod get_testrun_by_id {
        use super::*;

        #[tokio::test]
        async fn returns_none_when_missing() {
            let db = setup().await;
            let result = db.get_testrun_by_id(123).await.unwrap();
            assert!(result.is_none());
        }

        #[tokio::test]
        async fn returns_inserted_run_with_its_measurement() {
            let db = setup().await;
            seed_node(&db, 1).await;
            let mut run = minimal_test_run(1);
            run.error = Some("boom".to_string());

            let mut measurement = minimal_measurement(ExercisedInterface::MixForwarding);
            measurement.packets_sent = 42;
            measurement.packets_received = 41;
            let id = db.insert_test_run(&run, &[measurement]).await.unwrap().id;

            let fetched = db.get_testrun_by_id(id).await.unwrap().unwrap();
            assert_eq!(fetched.run.id, id);
            assert_eq!(fetched.run.inner.node_id, 1);
            assert_eq!(fetched.run.inner.test_kind, TestKind::Stress);
            assert_eq!(fetched.run.inner.tested_role, TestedRole::Mixnode);
            assert_eq!(fetched.run.inner.error.as_deref(), Some("boom"));

            let stored = fetched
                .measurement(ExercisedInterface::MixForwarding)
                .unwrap();
            assert_eq!(stored.packets_sent, 42);
            assert_eq!(stored.packets_received, 41);
        }

        #[tokio::test]
        async fn returns_the_right_row_when_multiple_exist() {
            let db = setup().await;
            seed_node(&db, 1).await;
            insert_run(&db, &minimal_test_run(1)).await;

            let mut measurement = minimal_measurement(ExercisedInterface::MixForwarding);
            measurement.packets_sent = 7;
            let target_id = db
                .insert_test_run(&minimal_test_run(1), &[measurement])
                .await
                .unwrap()
                .id;
            insert_run(&db, &minimal_test_run(1)).await;

            let fetched = db.get_testrun_by_id(target_id).await.unwrap().unwrap();
            assert_eq!(fetched.run.id, target_id);
            assert_eq!(
                fetched
                    .measurement(ExercisedInterface::MixForwarding)
                    .unwrap()
                    .packets_sent,
                7
            );
        }
    }

    mod get_latest_testrun_for_node {
        use super::*;

        #[tokio::test]
        async fn returns_none_when_never_tested() {
            let db = setup().await;
            seed_node(&db, 1).await;
            assert!(db.get_latest_testrun_for_node(1).await.unwrap().is_none());
        }

        #[tokio::test]
        async fn returns_the_newest_run_of_any_kind() {
            let db = setup().await;
            seed_node(&db, 1).await;
            seed_node(&db, 2).await;

            let mut older = minimal_test_run(1);
            older.test_timestamp = datetime!(2025-06-01 10:00:00 UTC);
            insert_run(&db, &older).await;

            let mut newest = minimal_test_run(1);
            newest.test_kind = TestKind::Liveness;
            newest.test_timestamp = datetime!(2025-06-01 12:00:00 UTC);
            let newest_id = insert_run(&db, &newest).await;

            // another node's newer run must not be picked up
            let mut other = minimal_test_run(2);
            other.test_timestamp = datetime!(2025-06-01 14:00:00 UTC);
            insert_run(&db, &other).await;

            let fetched = db.get_latest_testrun_for_node(1).await.unwrap().unwrap();
            assert_eq!(fetched.run.id, newest_id);
            assert_eq!(fetched.measurements.len(), 1);
        }
    }

    mod get_nym_node_by_id {
        use super::*;

        #[tokio::test]
        async fn returns_none_when_missing() {
            let db = setup().await;
            let result = db.get_nym_node_by_id(1).await.unwrap();
            assert!(result.is_none());
        }

        #[tokio::test]
        async fn returns_inserted_node() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(42, "key_a")])
                .await
                .unwrap();

            let fetched = db.get_nym_node_by_id(42).await.unwrap().unwrap();
            assert_eq!(fetched.inner.node_id, 42);
            assert_eq!(fetched.inner.identity_key, "key_a");
        }
    }

    mod get_testruns_in_progress_paginated {
        use super::*;

        #[tokio::test]
        async fn empty_when_table_empty() {
            let db = setup().await;
            let (rows, total) = db.get_testruns_in_progress_paginated(50, 0).await.unwrap();
            assert!(rows.is_empty());
            assert_eq!(total, 0);
        }

        #[tokio::test]
        async fn ordering_is_started_at_ascending() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[
                node(1, "key_a"),
                node(2, "key_b"),
                node(3, "key_c"),
            ])
            .await
            .unwrap();

            mark_in_progress(&db, 2, datetime!(2025-06-01 12:00:00 UTC)).await;
            mark_in_progress(&db, 3, datetime!(2025-06-01 10:00:00 UTC)).await;
            mark_in_progress(&db, 1, datetime!(2025-06-01 11:00:00 UTC)).await;

            let (rows, total) = db.get_testruns_in_progress_paginated(50, 0).await.unwrap();
            assert_eq!(total, 3);
            let ordered_node_ids: Vec<i64> = rows.iter().map(|r| r.node_id).collect();
            assert_eq!(ordered_node_ids, vec![3, 1, 2]);
        }

        #[tokio::test]
        async fn limit_truncates_page_but_preserves_total() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[
                node(1, "key_a"),
                node(2, "key_b"),
                node(3, "key_c"),
            ])
            .await
            .unwrap();

            mark_in_progress(&db, 1, datetime!(2025-06-01 10:00:00 UTC)).await;
            mark_in_progress(&db, 2, datetime!(2025-06-01 11:00:00 UTC)).await;
            mark_in_progress(&db, 3, datetime!(2025-06-01 12:00:00 UTC)).await;

            let (rows, total) = db.get_testruns_in_progress_paginated(2, 0).await.unwrap();
            assert_eq!(total, 3);
            let ordered_node_ids: Vec<i64> = rows.iter().map(|r| r.node_id).collect();
            assert_eq!(ordered_node_ids, vec![1, 2]);
        }

        #[tokio::test]
        async fn offset_skips_oldest_rows() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[
                node(1, "key_a"),
                node(2, "key_b"),
                node(3, "key_c"),
            ])
            .await
            .unwrap();

            mark_in_progress(&db, 1, datetime!(2025-06-01 10:00:00 UTC)).await;
            mark_in_progress(&db, 2, datetime!(2025-06-01 11:00:00 UTC)).await;
            mark_in_progress(&db, 3, datetime!(2025-06-01 12:00:00 UTC)).await;

            let (rows, total) = db.get_testruns_in_progress_paginated(2, 1).await.unwrap();
            assert_eq!(total, 3);
            let ordered_node_ids: Vec<i64> = rows.iter().map(|r| r.node_id).collect();
            assert_eq!(ordered_node_ids, vec![2, 3]);
        }

        #[tokio::test]
        async fn offset_past_end_returns_empty_but_accurate_total() {
            let db = setup().await;
            db.batch_insert_or_update_nym_nodes(&[node(1, "key_a"), node(2, "key_b")])
                .await
                .unwrap();

            mark_in_progress(&db, 1, datetime!(2025-06-01 10:00:00 UTC)).await;
            mark_in_progress(&db, 2, datetime!(2025-06-01 11:00:00 UTC)).await;

            let (rows, total) = db
                .get_testruns_in_progress_paginated(10, 100)
                .await
                .unwrap();
            assert!(rows.is_empty());
            assert_eq!(total, 2);
        }
    }

    mod get_nym_nodes_paginated {
        use super::*;

        #[tokio::test]
        async fn empty_when_table_empty() {
            let db = setup().await;
            let (rows, total) = db.get_nym_nodes_paginated(50, 0).await.unwrap();
            assert!(rows.is_empty());
            assert_eq!(total, 0);
        }

        #[tokio::test]
        async fn returns_first_page_and_correct_total() {
            let db = setup().await;
            let nodes: Vec<NewNymNode> = (1..=5).map(|i| node(i, &format!("key_{i}"))).collect();
            db.batch_insert_or_update_nym_nodes(&nodes).await.unwrap();

            let (rows, total) = db.get_nym_nodes_paginated(2, 0).await.unwrap();
            assert_eq!(total, 5);
            let ids: Vec<i64> = rows.iter().map(|r| r.inner.node_id).collect();
            assert_eq!(ids, vec![1, 2]);
        }

        #[tokio::test]
        async fn offset_skips_earlier_rows() {
            let db = setup().await;
            let nodes: Vec<NewNymNode> = (1..=5).map(|i| node(i, &format!("key_{i}"))).collect();
            db.batch_insert_or_update_nym_nodes(&nodes).await.unwrap();

            let (rows, total) = db.get_nym_nodes_paginated(2, 2).await.unwrap();
            assert_eq!(total, 5);
            let ids: Vec<i64> = rows.iter().map(|r| r.inner.node_id).collect();
            assert_eq!(ids, vec![3, 4]);
        }

        #[tokio::test]
        async fn offset_past_end_returns_empty_but_accurate_total() {
            let db = setup().await;
            let nodes: Vec<NewNymNode> = (1..=3).map(|i| node(i, &format!("key_{i}"))).collect();
            db.batch_insert_or_update_nym_nodes(&nodes).await.unwrap();

            let (rows, total) = db.get_nym_nodes_paginated(10, 100).await.unwrap();
            assert!(rows.is_empty());
            assert_eq!(total, 3);
        }

        #[tokio::test]
        async fn ordering_is_node_id_ascending() {
            let db = setup().await;
            // insert in non-ascending order to confirm ORDER BY actually sorts
            db.batch_insert_or_update_nym_nodes(&[
                node(3, "key_c"),
                node(1, "key_a"),
                node(2, "key_b"),
            ])
            .await
            .unwrap();

            let (rows, _) = db.get_nym_nodes_paginated(10, 0).await.unwrap();
            let ids: Vec<i64> = rows.iter().map(|r| r.inner.node_id).collect();
            assert_eq!(ids, vec![1, 2, 3]);
        }
    }

    mod get_testruns_paginated {
        use super::*;

        async fn insert_run_at(db: &StorageManager, node_id: i64, ts: OffsetDateTime) -> i64 {
            let mut run = minimal_test_run(node_id);
            run.test_timestamp = ts;
            insert_run(db, &run).await
        }

        #[tokio::test]
        async fn empty_when_table_empty() {
            let db = setup().await;
            let (rows, total) = db.get_testruns_paginated(50, 0).await.unwrap();
            assert!(rows.is_empty());
            assert_eq!(total, 0);
        }

        #[tokio::test]
        async fn ordering_is_test_timestamp_descending() {
            let db = setup().await;
            seed_node(&db, 1).await;
            // insert in mixed order; ensure query returns newest first
            insert_run_at(&db, 1, datetime!(2025-03-01 00:00:00 UTC)).await;
            insert_run_at(&db, 1, datetime!(2025-01-01 00:00:00 UTC)).await;
            insert_run_at(&db, 1, datetime!(2025-02-01 00:00:00 UTC)).await;

            let (rows, total) = db.get_testruns_paginated(10, 0).await.unwrap();
            assert_eq!(total, 3);
            let timestamps: Vec<OffsetDateTime> =
                rows.iter().map(|r| r.run.inner.test_timestamp).collect();
            assert_eq!(
                timestamps,
                vec![
                    datetime!(2025-03-01 00:00:00 UTC),
                    datetime!(2025-02-01 00:00:00 UTC),
                    datetime!(2025-01-01 00:00:00 UTC),
                ]
            );
        }

        // the page and its measurements are two queries, so pin that every run comes back with its
        // own rather than with another run's
        #[tokio::test]
        async fn every_run_in_a_page_carries_its_own_measurements() {
            let db = setup().await;
            seed_node(&db, 1).await;

            for (packets, ts) in [
                (10, datetime!(2025-01-01 00:00:00 UTC)),
                (20, datetime!(2025-02-01 00:00:00 UTC)),
                (30, datetime!(2025-03-01 00:00:00 UTC)),
            ] {
                let mut run = minimal_test_run(1);
                run.test_timestamp = ts;
                let mut measurement = minimal_measurement(ExercisedInterface::MixForwarding);
                measurement.packets_sent = packets;
                db.insert_test_run(&run, &[measurement]).await.unwrap();
            }

            let (rows, _) = db.get_testruns_paginated(10, 0).await.unwrap();
            let sent: Vec<i64> = rows
                .iter()
                .map(|r| {
                    r.measurement(ExercisedInterface::MixForwarding)
                        .unwrap()
                        .packets_sent
                })
                .collect();
            // newest first, so the measurements must be in the reverse of the insertion order
            assert_eq!(sent, vec![30, 20, 10]);
        }

        #[tokio::test]
        async fn offset_skips_newest_rows() {
            let db = setup().await;
            seed_node(&db, 1).await;
            insert_run_at(&db, 1, datetime!(2025-03-01 00:00:00 UTC)).await;
            insert_run_at(&db, 1, datetime!(2025-02-01 00:00:00 UTC)).await;
            insert_run_at(&db, 1, datetime!(2025-01-01 00:00:00 UTC)).await;

            let (rows, total) = db.get_testruns_paginated(2, 1).await.unwrap();
            assert_eq!(total, 3);
            let timestamps: Vec<OffsetDateTime> =
                rows.iter().map(|r| r.run.inner.test_timestamp).collect();
            assert_eq!(
                timestamps,
                vec![
                    datetime!(2025-02-01 00:00:00 UTC),
                    datetime!(2025-01-01 00:00:00 UTC),
                ]
            );
        }

        #[tokio::test]
        async fn offset_past_end_returns_empty_but_accurate_total() {
            let db = setup().await;
            seed_node(&db, 1).await;
            insert_run(&db, &minimal_test_run(1)).await;
            insert_run(&db, &minimal_test_run(1)).await;

            let (rows, total) = db.get_testruns_paginated(10, 50).await.unwrap();
            assert!(rows.is_empty());
            assert_eq!(total, 2);
        }
    }

    mod get_testruns_for_node_paginated {
        use super::*;

        async fn insert_run_at(db: &StorageManager, node_id: i64, ts: OffsetDateTime) -> i64 {
            let mut run = minimal_test_run(node_id);
            run.test_timestamp = ts;
            insert_run(db, &run).await
        }

        #[tokio::test]
        async fn empty_when_node_has_no_runs() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let (rows, total) = db.get_testruns_for_node_paginated(1, 50, 0).await.unwrap();
            assert!(rows.is_empty());
            assert_eq!(total, 0);
        }

        #[tokio::test]
        async fn returns_only_runs_for_requested_node() {
            let db = setup().await;
            seed_node(&db, 1).await;
            seed_node(&db, 2).await;

            insert_run(&db, &minimal_test_run(1)).await;
            insert_run(&db, &minimal_test_run(1)).await;
            insert_run(&db, &minimal_test_run(2)).await;

            let (rows, total) = db.get_testruns_for_node_paginated(1, 50, 0).await.unwrap();
            assert_eq!(total, 2);
            assert_eq!(rows.len(), 2);
            assert!(rows.iter().all(|r| r.run.inner.node_id == 1));

            let (rows, total) = db.get_testruns_for_node_paginated(2, 50, 0).await.unwrap();
            assert_eq!(total, 1);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].run.inner.node_id, 2);
        }

        #[tokio::test]
        async fn ordering_is_test_timestamp_descending() {
            let db = setup().await;
            seed_node(&db, 1).await;

            insert_run_at(&db, 1, datetime!(2025-02-01 00:00:00 UTC)).await;
            insert_run_at(&db, 1, datetime!(2025-03-01 00:00:00 UTC)).await;
            insert_run_at(&db, 1, datetime!(2025-01-01 00:00:00 UTC)).await;

            let (rows, _) = db.get_testruns_for_node_paginated(1, 10, 0).await.unwrap();
            let timestamps: Vec<OffsetDateTime> =
                rows.iter().map(|r| r.run.inner.test_timestamp).collect();
            assert_eq!(
                timestamps,
                vec![
                    datetime!(2025-03-01 00:00:00 UTC),
                    datetime!(2025-02-01 00:00:00 UTC),
                    datetime!(2025-01-01 00:00:00 UTC),
                ]
            );
        }

        #[tokio::test]
        async fn offset_skips_newest_rows() {
            let db = setup().await;
            seed_node(&db, 1).await;

            insert_run_at(&db, 1, datetime!(2025-03-01 00:00:00 UTC)).await;
            insert_run_at(&db, 1, datetime!(2025-02-01 00:00:00 UTC)).await;
            insert_run_at(&db, 1, datetime!(2025-01-01 00:00:00 UTC)).await;

            let (rows, total) = db.get_testruns_for_node_paginated(1, 2, 1).await.unwrap();
            assert_eq!(total, 3);
            let timestamps: Vec<OffsetDateTime> =
                rows.iter().map(|r| r.run.inner.test_timestamp).collect();
            assert_eq!(
                timestamps,
                vec![
                    datetime!(2025-02-01 00:00:00 UTC),
                    datetime!(2025-01-01 00:00:00 UTC),
                ]
            );
        }

        #[tokio::test]
        async fn unknown_node_returns_empty_with_zero_total() {
            let db = setup().await;
            seed_node(&db, 1).await;
            insert_run(&db, &minimal_test_run(1)).await;

            // node 99 was never seeded, so it has no runs and total is 0.
            let (rows, total) = db.get_testruns_for_node_paginated(99, 50, 0).await.unwrap();
            assert!(rows.is_empty());
            assert_eq!(total, 0);
        }
    }

    mod submission_watermark {
        use super::*;

        #[tokio::test]
        async fn absent_until_a_batch_is_submitted() {
            let db = setup().await;
            assert!(
                db.get_last_submitted_testrun_id(TestKind::Stress)
                    .await
                    .unwrap()
                    .is_none()
            );
        }

        #[tokio::test]
        async fn round_trips_and_overwrites() {
            let db = setup().await;
            db.set_last_submitted_testrun_id(TestKind::Stress, 7)
                .await
                .unwrap();
            assert_eq!(
                db.get_last_submitted_testrun_id(TestKind::Stress)
                    .await
                    .unwrap(),
                Some(7)
            );

            db.set_last_submitted_testrun_id(TestKind::Stress, 9)
                .await
                .unwrap();
            assert_eq!(
                db.get_last_submitted_testrun_id(TestKind::Stress)
                    .await
                    .unwrap(),
                Some(9)
            );
        }

        // the two streams post to different endpoints, so advancing one must leave the other where
        // it was - a shared value would drag unsubmitted rows past the watermark
        #[tokio::test]
        async fn each_kind_keeps_its_own_position() {
            let db = setup().await;
            db.set_last_submitted_testrun_id(TestKind::Stress, 7)
                .await
                .unwrap();
            db.set_last_submitted_testrun_id(TestKind::Liveness, 3)
                .await
                .unwrap();

            assert_eq!(
                db.get_last_submitted_testrun_id(TestKind::Stress)
                    .await
                    .unwrap(),
                Some(7)
            );
            assert_eq!(
                db.get_last_submitted_testrun_id(TestKind::Liveness)
                    .await
                    .unwrap(),
                Some(3)
            );
        }
    }

    mod get_testruns_after {
        use super::*;

        #[tokio::test]
        async fn returns_everything_when_nothing_submitted() {
            let db = setup().await;
            seed_node(&db, 1).await;
            insert_run(&db, &minimal_test_run(1)).await;
            insert_run(&db, &minimal_test_run(1)).await;

            let pending = db.get_testruns_after(TestKind::Stress, 0).await.unwrap();
            assert_eq!(pending.len(), 2);
            assert!(pending.iter().all(|run| run.measurements.len() == 1));
        }

        #[tokio::test]
        async fn skips_rows_at_or_below_the_watermark() {
            let db = setup().await;
            seed_node(&db, 1).await;
            let first = insert_run(&db, &minimal_test_run(1)).await;
            let second = insert_run(&db, &minimal_test_run(1)).await;

            let pending = db
                .get_testruns_after(TestKind::Stress, first)
                .await
                .unwrap();
            let ids: Vec<i64> = pending.iter().map(|run| run.run.id).collect();
            assert_eq!(ids, vec![second]);
        }

        // each kind is submitted to its own endpoint, so a stream must never pick up the other's
        // rows
        #[tokio::test]
        async fn returns_only_the_requested_kind() {
            let db = setup().await;
            seed_node(&db, 1).await;
            let stress_id = insert_run(&db, &minimal_test_run(1)).await;

            let mut liveness = minimal_test_run(1);
            liveness.test_kind = TestKind::Liveness;
            let liveness_id = insert_run(&db, &liveness).await;

            let stress = db.get_testruns_after(TestKind::Stress, 0).await.unwrap();
            assert_eq!(
                stress.iter().map(|run| run.run.id).collect::<Vec<_>>(),
                vec![stress_id]
            );

            let liveness = db.get_testruns_after(TestKind::Liveness, 0).await.unwrap();
            assert_eq!(
                liveness.iter().map(|run| run.run.id).collect::<Vec<_>>(),
                vec![liveness_id]
            );
        }
    }
}
