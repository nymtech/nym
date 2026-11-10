// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::models::{
    AssignedTestrun, AssignmentCandidate, AssignmentRequest, BondedNymNode, CompletedTestRun,
    ConfigScoreCandidate, GatewayLivenessTestRunRow, MixnetEpochAggregate, MixnetEpochConfigScore,
    MixnodeTestRunRow, NewTestRun, NodeAwaitingCapabilityRefresh, NodeChainCapability,
    NodeDescription, NymNode, TestKind, TestRunInProgress, TestRunSubmission, TestRunWindow,
    duration_to_us, next_ip_to_test,
};
use nym_network_monitor_orchestrator_requests::models::{InterfaceMeasurement, RunMeasurements};
use sqlx::SqliteConnection;
use std::collections::HashMap;
use strum::IntoEnumIterator;
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
                    s.last_tested_at AS "last_tested_at?"
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
                    s.last_tested_at AS "last_tested_at?"
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

/// Stores `run` in its kind's results table and records it as that kind's latest test of the node,
/// returning the id it was stored under.
///
/// The table is chosen by the shape of the measurements. The kind's rotation pointer is
/// deliberately not touched: it belongs to the assignment, which advances it when the work is
/// handed out so that an abandoned run still moves the node onto its next address.
async fn record_testrun(
    conn: &mut SqliteConnection,
    run: &NewTestRun,
    measurements: &RunMeasurements,
) -> anyhow::Result<i64> {
    let id = match measurements {
        RunMeasurements::MixnodeLiveness { mix_forwarding } => {
            insert_mixnode_liveness_testrun(&mut *conn, run, mix_forwarding).await?
        }
        RunMeasurements::GatewayLiveness {
            client_ingest,
            client_delivery,
        } => {
            insert_gateway_liveness_testrun(&mut *conn, run, client_ingest, client_delivery).await?
        }
        RunMeasurements::MixnodeStress { mix_forwarding } => {
            insert_mixnode_stress_testrun(&mut *conn, run, mix_forwarding).await?
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
    .execute(conn)
    .await?;

    Ok(id)
}

// One kind's single-statement reads and deletes. The free functions around this block take a
// connection instead because they run inside a transaction their caller owns.
impl StorageManager {
    /// Every `mixnode_liveness` run with an id above `after_id`, oldest id first.
    async fn get_mixnode_liveness_testruns_after(
        &self,
        after_id: i64,
    ) -> anyhow::Result<Vec<CompletedTestRun>> {
        let rows = sqlx::query_as!(
            MixnodeTestRunRow,
            r#"
            SELECT *
            FROM mixnode_liveness_testrun
            WHERE id > ?
            ORDER BY id ASC
            "#,
            after_id
        )
        .fetch_all(&self.connection_pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(MixnodeTestRunRow::into_mixnode_liveness)
            .collect())
    }

    /// Every `gateway_liveness` run with an id above `after_id`, oldest id first.
    async fn get_gateway_liveness_testruns_after(
        &self,
        after_id: i64,
    ) -> anyhow::Result<Vec<CompletedTestRun>> {
        let rows = sqlx::query_as!(
            GatewayLivenessTestRunRow,
            r#"
            SELECT *
            FROM gateway_liveness_testrun
            WHERE id > ?
            ORDER BY id ASC
            "#,
            after_id
        )
        .fetch_all(&self.connection_pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(GatewayLivenessTestRunRow::into_gateway_liveness)
            .collect())
    }

    /// Every `mixnode_stress` run with an id above `after_id`, oldest id first.
    async fn get_mixnode_stress_testruns_after(
        &self,
        after_id: i64,
    ) -> anyhow::Result<Vec<CompletedTestRun>> {
        let rows = sqlx::query_as!(
            MixnodeTestRunRow,
            r#"
            SELECT *
            FROM mixnode_stress_testrun
            WHERE id > ?
            ORDER BY id ASC
            "#,
            after_id
        )
        .fetch_all(&self.connection_pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(MixnodeTestRunRow::into_mixnode_stress)
            .collect())
    }

    /// Every `mixnode_liveness` run stored within `window`.
    async fn get_mixnode_liveness_testruns_in_window(
        &self,
        window: TestRunWindow,
    ) -> anyhow::Result<Vec<CompletedTestRun>> {
        let rows = sqlx::query_as!(
            MixnodeTestRunRow,
            r#"
            SELECT *
            FROM mixnode_liveness_testrun
            WHERE test_timestamp >= ? AND test_timestamp < ?
            "#,
            window.start,
            window.end
        )
        .fetch_all(&self.connection_pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(MixnodeTestRunRow::into_mixnode_liveness)
            .collect())
    }

    /// Every `gateway_liveness` run stored within `window`.
    async fn get_gateway_liveness_testruns_in_window(
        &self,
        window: TestRunWindow,
    ) -> anyhow::Result<Vec<CompletedTestRun>> {
        let rows = sqlx::query_as!(
            GatewayLivenessTestRunRow,
            r#"
            SELECT *
            FROM gateway_liveness_testrun
            WHERE test_timestamp >= ? AND test_timestamp < ?
            "#,
            window.start,
            window.end
        )
        .fetch_all(&self.connection_pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(GatewayLivenessTestRunRow::into_gateway_liveness)
            .collect())
    }

    /// Every `mixnode_stress` run stored within `window`.
    async fn get_mixnode_stress_testruns_in_window(
        &self,
        window: TestRunWindow,
    ) -> anyhow::Result<Vec<CompletedTestRun>> {
        let rows = sqlx::query_as!(
            MixnodeTestRunRow,
            r#"
            SELECT *
            FROM mixnode_stress_testrun
            WHERE test_timestamp >= ? AND test_timestamp < ?
            "#,
            window.start,
            window.end
        )
        .fetch_all(&self.connection_pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(MixnodeTestRunRow::into_mixnode_stress)
            .collect())
    }

    /// The `mixnode_liveness` run stored under `id`, if it still exists.
    async fn get_mixnode_liveness_testrun_by_id(
        &self,
        id: i64,
    ) -> anyhow::Result<Option<CompletedTestRun>> {
        let row = sqlx::query_as!(
            MixnodeTestRunRow,
            r#"
            SELECT *
            FROM mixnode_liveness_testrun
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        Ok(row.map(MixnodeTestRunRow::into_mixnode_liveness))
    }

    /// The `gateway_liveness` run stored under `id`, if it still exists.
    async fn get_gateway_liveness_testrun_by_id(
        &self,
        id: i64,
    ) -> anyhow::Result<Option<CompletedTestRun>> {
        let row = sqlx::query_as!(
            GatewayLivenessTestRunRow,
            r#"
            SELECT *
            FROM gateway_liveness_testrun
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        Ok(row.map(GatewayLivenessTestRunRow::into_gateway_liveness))
    }

    /// The `mixnode_stress` run stored under `id`, if it still exists.
    async fn get_mixnode_stress_testrun_by_id(
        &self,
        id: i64,
    ) -> anyhow::Result<Option<CompletedTestRun>> {
        let row = sqlx::query_as!(
            MixnodeTestRunRow,
            r#"
            SELECT *
            FROM mixnode_stress_testrun
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.connection_pool)
        .await?;

        Ok(row.map(MixnodeTestRunRow::into_mixnode_stress))
    }

    /// Deletes every `mixnode_liveness` run older than `cutoff`, returning how many went.
    async fn evict_old_mixnode_liveness_testruns(
        &self,
        cutoff: OffsetDateTime,
    ) -> anyhow::Result<u64> {
        let evicted = sqlx::query!(
            "DELETE FROM mixnode_liveness_testrun WHERE test_timestamp < ?",
            cutoff
        )
        .execute(&self.connection_pool)
        .await?
        .rows_affected();
        Ok(evicted)
    }

    /// Deletes every `gateway_liveness` run older than `cutoff`, returning how many went.
    async fn evict_old_gateway_liveness_testruns(
        &self,
        cutoff: OffsetDateTime,
    ) -> anyhow::Result<u64> {
        let evicted = sqlx::query!(
            "DELETE FROM gateway_liveness_testrun WHERE test_timestamp < ?",
            cutoff
        )
        .execute(&self.connection_pool)
        .await?
        .rows_affected();
        Ok(evicted)
    }

    /// Deletes every `mixnode_stress` run older than `cutoff`, returning how many went.
    async fn evict_old_mixnode_stress_testruns(
        &self,
        cutoff: OffsetDateTime,
    ) -> anyhow::Result<u64> {
        let evicted = sqlx::query!(
            "DELETE FROM mixnode_stress_testrun WHERE test_timestamp < ?",
            cutoff
        )
        .execute(&self.connection_pool)
        .await?
        .rows_affected();
        Ok(evicted)
    }
}

/// A page of `mixnode_liveness` runs, newest first, with the table's total row count.
async fn get_mixnode_liveness_testruns_page(
    conn: &mut SqliteConnection,
    limit: i64,
    offset: i64,
) -> anyhow::Result<(Vec<CompletedTestRun>, i64)> {
    let rows = sqlx::query_as!(
        MixnodeTestRunRow,
        r#"
            SELECT *
            FROM mixnode_liveness_testrun
            ORDER BY test_timestamp DESC
            LIMIT ? OFFSET ?
        "#,
        limit,
        offset
    )
    .fetch_all(&mut *conn)
    .await?;

    let total = sqlx::query_scalar!("SELECT COUNT(*) FROM mixnode_liveness_testrun")
        .fetch_one(&mut *conn)
        .await?;

    let runs = rows
        .into_iter()
        .map(MixnodeTestRunRow::into_mixnode_liveness)
        .collect();
    Ok((runs, total))
}

/// A page of `gateway_liveness` runs, newest first, with the table's total row count.
async fn get_gateway_liveness_testruns_page(
    conn: &mut SqliteConnection,
    limit: i64,
    offset: i64,
) -> anyhow::Result<(Vec<CompletedTestRun>, i64)> {
    let rows = sqlx::query_as!(
        GatewayLivenessTestRunRow,
        r#"
            SELECT *
            FROM gateway_liveness_testrun
            ORDER BY test_timestamp DESC
            LIMIT ? OFFSET ?
        "#,
        limit,
        offset
    )
    .fetch_all(&mut *conn)
    .await?;

    let total = sqlx::query_scalar!("SELECT COUNT(*) FROM gateway_liveness_testrun")
        .fetch_one(&mut *conn)
        .await?;

    let runs = rows
        .into_iter()
        .map(GatewayLivenessTestRunRow::into_gateway_liveness)
        .collect();
    Ok((runs, total))
}

/// A page of `mixnode_stress` runs, newest first, with the table's total row count.
async fn get_mixnode_stress_testruns_page(
    conn: &mut SqliteConnection,
    limit: i64,
    offset: i64,
) -> anyhow::Result<(Vec<CompletedTestRun>, i64)> {
    let rows = sqlx::query_as!(
        MixnodeTestRunRow,
        r#"
            SELECT *
            FROM mixnode_stress_testrun
            ORDER BY test_timestamp DESC
            LIMIT ? OFFSET ?
        "#,
        limit,
        offset
    )
    .fetch_all(&mut *conn)
    .await?;

    let total = sqlx::query_scalar!("SELECT COUNT(*) FROM mixnode_stress_testrun")
        .fetch_one(&mut *conn)
        .await?;

    let runs = rows
        .into_iter()
        .map(MixnodeTestRunRow::into_mixnode_stress)
        .collect();
    Ok((runs, total))
}

/// A page of the `mixnode_liveness` runs against `node_id`, newest first, with that node's total
/// run count in the table.
async fn get_mixnode_liveness_testruns_for_node_page(
    conn: &mut SqliteConnection,
    node_id: i64,
    limit: i64,
    offset: i64,
) -> anyhow::Result<(Vec<CompletedTestRun>, i64)> {
    let rows = sqlx::query_as!(
        MixnodeTestRunRow,
        r#"
            SELECT *
            FROM mixnode_liveness_testrun
            WHERE node_id = ?
            ORDER BY test_timestamp DESC
            LIMIT ? OFFSET ?
        "#,
        node_id,
        limit,
        offset
    )
    .fetch_all(&mut *conn)
    .await?;

    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM mixnode_liveness_testrun WHERE node_id = ?",
        node_id
    )
    .fetch_one(&mut *conn)
    .await?;

    let runs = rows
        .into_iter()
        .map(MixnodeTestRunRow::into_mixnode_liveness)
        .collect();
    Ok((runs, total))
}

/// A page of the `gateway_liveness` runs against `node_id`, newest first, with that node's total
/// run count in the table.
async fn get_gateway_liveness_testruns_for_node_page(
    conn: &mut SqliteConnection,
    node_id: i64,
    limit: i64,
    offset: i64,
) -> anyhow::Result<(Vec<CompletedTestRun>, i64)> {
    let rows = sqlx::query_as!(
        GatewayLivenessTestRunRow,
        r#"
            SELECT *
            FROM gateway_liveness_testrun
            WHERE node_id = ?
            ORDER BY test_timestamp DESC
            LIMIT ? OFFSET ?
        "#,
        node_id,
        limit,
        offset
    )
    .fetch_all(&mut *conn)
    .await?;

    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM gateway_liveness_testrun WHERE node_id = ?",
        node_id
    )
    .fetch_one(&mut *conn)
    .await?;

    let runs = rows
        .into_iter()
        .map(GatewayLivenessTestRunRow::into_gateway_liveness)
        .collect();
    Ok((runs, total))
}

/// A page of the `mixnode_stress` runs against `node_id`, newest first, with that node's total run
/// count in the table.
async fn get_mixnode_stress_testruns_for_node_page(
    conn: &mut SqliteConnection,
    node_id: i64,
    limit: i64,
    offset: i64,
) -> anyhow::Result<(Vec<CompletedTestRun>, i64)> {
    let rows = sqlx::query_as!(
        MixnodeTestRunRow,
        r#"
            SELECT *
            FROM mixnode_stress_testrun
            WHERE node_id = ?
            ORDER BY test_timestamp DESC
            LIMIT ? OFFSET ?
        "#,
        node_id,
        limit,
        offset
    )
    .fetch_all(&mut *conn)
    .await?;

    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM mixnode_stress_testrun WHERE node_id = ?",
        node_id
    )
    .fetch_one(&mut *conn)
    .await?;

    let runs = rows
        .into_iter()
        .map(MixnodeTestRunRow::into_mixnode_stress)
        .collect();
    Ok((runs, total))
}

/// The bond of `node_id`, if the orchestrator has ever seen it.
async fn get_node_bond(
    conn: &mut SqliteConnection,
    node_id: i64,
) -> anyhow::Result<Option<BondedNymNode>> {
    let bond = sqlx::query_as!(
        BondedNymNode,
        "SELECT * FROM nym_node_bond WHERE node_id = ?",
        node_id
    )
    .fetch_optional(conn)
    .await?;
    Ok(bond)
}

/// The description of `node_id`, if it currently has one.
async fn get_node_description(
    conn: &mut SqliteConnection,
    node_id: i64,
) -> anyhow::Result<Option<NodeDescription>> {
    let description = sqlx::query_as!(
        NodeDescription,
        r#"
        SELECT
            mix_port,
            announced_ips,
            noise_key,
            sphinx_key,
            key_rotation_id,
            mixnode_enabled,
            gateway_enabled,
            clients_ws_port,
            reported_version,
            binary_name,
            accepted_terms_and_conditions,
            declared_chain_address
        FROM nym_node_description
        WHERE node_id = ?
        "#,
        node_id
    )
    .fetch_optional(conn)
    .await?;
    Ok(description)
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
        nodes: &[NymNode],
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
                    clients_ws_port,
                    reported_version,
                    binary_name,
                    accepted_terms_and_conditions,
                    declared_chain_address
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (node_id) DO UPDATE SET
                    mix_port                      = excluded.mix_port,
                    announced_ips                 = excluded.announced_ips,
                    noise_key                     = excluded.noise_key,
                    sphinx_key                    = excluded.sphinx_key,
                    key_rotation_id               = excluded.key_rotation_id,
                    mixnode_enabled               = excluded.mixnode_enabled,
                    gateway_enabled               = excluded.gateway_enabled,
                    clients_ws_port               = excluded.clients_ws_port,
                    reported_version              = excluded.reported_version,
                    binary_name                   = excluded.binary_name,
                    accepted_terms_and_conditions = excluded.accepted_terms_and_conditions,
                    declared_chain_address        = excluded.declared_chain_address
                "#,
                bond.node_id,
                description.mix_port,
                description.announced_ips,
                description.noise_key,
                description.sphinx_key,
                description.key_rotation_id,
                description.mixnode_enabled,
                description.gateway_enabled,
                description.clients_ws_port,
                description.reported_version,
                description.binary_name,
                description.accepted_terms_and_conditions,
                description.declared_chain_address,
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

    /// Records a submitted result against the run the orchestrator dispatched for its node, in ONE
    /// transaction: checks the node's in-flight row, stores the run in its kind's results table,
    /// records that kind's work state and releases the lock.
    ///
    /// Nothing is stored when the node has no in-flight row, because its lease expired and the
    /// sweep freed it, or when the row was dispatched for a different kind: the row is the
    /// authoritative record of what was asked for. `BEGIN IMMEDIATE` because the transaction reads
    /// before it writes, so the row it checks is the row it releases.
    pub(crate) async fn submit_testrun(
        &self,
        run: &NewTestRun,
        measurements: &RunMeasurements,
    ) -> anyhow::Result<TestRunSubmission> {
        let mut tx = self.connection_pool.begin_with("BEGIN IMMEDIATE").await?;

        let dispatched = sqlx::query_scalar!(
            r#"SELECT test_kind AS "test_kind: TestKind" FROM testrun_in_progress WHERE node_id = ?"#,
            run.node_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        match dispatched {
            None => return Ok(TestRunSubmission::LeaseExpired),
            Some(dispatched) if dispatched != TestKind::from(measurements.kind()) => {
                return Ok(TestRunSubmission::UnexpectedKind { dispatched });
            }
            Some(_) => {}
        }

        record_testrun(&mut tx, run, measurements).await?;
        sqlx::query!(
            "DELETE FROM testrun_in_progress WHERE node_id = ?",
            run.node_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(TestRunSubmission::Stored)
    }

    /// Stores a run with no in-flight check, for tests that need results without dispatching them
    /// first. Returns the id it was stored under.
    #[cfg(test)]
    pub(crate) async fn insert_test_run(
        &self,
        run: &NewTestRun,
        measurements: &RunMeasurements,
    ) -> anyhow::Result<i64> {
        let mut tx = self.connection_pool.begin().await?;
        let id = record_testrun(&mut tx, run, measurements).await?;
        tx.commit().await?;
        Ok(id)
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

    /// The in-flight row for a node, or `None` if it has none.
    #[cfg(test)]
    pub(crate) async fn get_testrun_in_progress(
        &self,
        node_id: i64,
    ) -> anyhow::Result<Option<TestRunInProgress>> {
        let row = sqlx::query_as!(
            TestRunInProgress,
            r#"
            SELECT node_id, started_at, expires_at, test_kind AS "test_kind: TestKind"
            FROM testrun_in_progress
            WHERE node_id = ?
            "#,
            node_id
        )
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

        let expiring = sqlx::query!(
            r#"
            SELECT test_kind AS "test_kind: TestKind", COUNT(*) AS "count!: i64"
            FROM testrun_in_progress
            WHERE expires_at < ?
            GROUP BY test_kind
            "#,
            now
        )
        .fetch_all(&mut *tx)
        .await?;

        sqlx::query!("DELETE FROM testrun_in_progress WHERE expires_at < ?", now,)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(expiring
            .into_iter()
            .map(|row| (row.test_kind, row.count as u64))
            .collect())
    }

    /// Returns the number of rows currently in `testrun_in_progress` - i.e. the number of
    /// test runs that have been assigned to an agent but not yet submitted back.
    pub(crate) async fn count_testruns_in_progress(&self) -> anyhow::Result<i64> {
        let total = sqlx::query_scalar!("SELECT COUNT(*) FROM testrun_in_progress")
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
        let counts = sqlx::query!(
            r#"
            SELECT test_kind AS "test_kind: TestKind", COUNT(*) AS "count!: i64"
            FROM testrun_in_progress
            GROUP BY test_kind
            "#
        )
        .fetch_all(&self.connection_pool)
        .await?;

        Ok(counts
            .into_iter()
            .map(|row| (row.test_kind, row.count))
            .collect())
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

    /// Fetches one completed run of `test_kind` by its id within that kind. Returns `None` if no
    /// such run exists.
    pub(crate) async fn get_testrun_by_id(
        &self,
        test_kind: TestKind,
        id: i64,
    ) -> anyhow::Result<Option<CompletedTestRun>> {
        match test_kind {
            TestKind::MixnodeLiveness => self.get_mixnode_liveness_testrun_by_id(id).await,
            TestKind::GatewayLiveness => self.get_gateway_liveness_testrun_by_id(id).await,
            TestKind::MixnodeStress => self.get_mixnode_stress_testrun_by_id(id).await,
        }
    }

    /// Fetches the newest completed run of `test_kind` against a node, or `None` if that kind has
    /// never tested it (or its runs have all been evicted). The first entry of the node's first
    /// page, so it cannot disagree with the paginated read.
    pub(crate) async fn get_latest_testrun_for_node(
        &self,
        test_kind: TestKind,
        node_id: i64,
    ) -> anyhow::Result<Option<CompletedTestRun>> {
        let (runs, _total) = self
            .get_testruns_for_node_paginated(test_kind, node_id, 1, 0)
            .await?;
        Ok(runs.into_iter().next())
    }

    /// Fetches a node by its `node_id`, with its description if it currently has one.
    ///
    /// Returns `None` if the orchestrator has never seen a bond for this node.
    pub(crate) async fn get_nym_node_by_id(&self, node_id: i64) -> anyhow::Result<Option<NymNode>> {
        let mut tx = self.connection_pool.begin().await?;

        let Some(bond) = get_node_bond(&mut tx, node_id).await? else {
            return Ok(None);
        };
        let description = get_node_description(&mut tx, node_id).await?;

        tx.commit().await?;
        Ok(Some(NymNode { bond, description }))
    }

    /// Fetches a page of the completed runs of `test_kind` against a single `node_id`, newest
    /// first, together with that node's total run count for the kind (used to populate
    /// `PagedResult::total`).
    ///
    /// `limit` and `offset` translate directly to SQL `LIMIT` / `OFFSET`; the caller is
    /// expected to derive them from the public pagination contract as
    /// `limit = size` and `offset = page * size`.
    ///
    /// The page and the total share one transaction (no tearing if another writer commits in
    /// between).
    pub(crate) async fn get_testruns_for_node_paginated(
        &self,
        test_kind: TestKind,
        node_id: i64,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(Vec<CompletedTestRun>, i64)> {
        let mut tx = self.connection_pool.begin().await?;
        let page = match test_kind {
            TestKind::MixnodeLiveness => {
                get_mixnode_liveness_testruns_for_node_page(&mut tx, node_id, limit, offset).await?
            }
            TestKind::GatewayLiveness => {
                get_gateway_liveness_testruns_for_node_page(&mut tx, node_id, limit, offset).await?
            }
            TestKind::MixnodeStress => {
                get_mixnode_stress_testruns_for_node_page(&mut tx, node_id, limit, offset).await?
            }
        };
        tx.commit().await?;
        Ok(page)
    }

    /// Fetches a page of the completed runs of `test_kind`, newest first, together with the kind's
    /// total run count (used to populate `PagedResult::total`).
    ///
    /// `limit` and `offset` translate directly to SQL `LIMIT` / `OFFSET`; the caller is
    /// expected to derive them from the public pagination contract as
    /// `limit = size` and `offset = page * size`.
    ///
    /// The page and the total share one transaction (no tearing if another writer commits in
    /// between).
    pub(crate) async fn get_testruns_paginated(
        &self,
        test_kind: TestKind,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(Vec<CompletedTestRun>, i64)> {
        let mut tx = self.connection_pool.begin().await?;
        let page = match test_kind {
            TestKind::MixnodeLiveness => {
                get_mixnode_liveness_testruns_page(&mut tx, limit, offset).await?
            }
            TestKind::GatewayLiveness => {
                get_gateway_liveness_testruns_page(&mut tx, limit, offset).await?
            }
            TestKind::MixnodeStress => {
                get_mixnode_stress_testruns_page(&mut tx, limit, offset).await?
            }
        };
        tx.commit().await?;
        Ok(page)
    }

    /// Fetches a page of nodes, ordered by `node_id` ascending, each with its description if it
    /// currently has one, together with the total number of bonds (used to populate
    /// `PagedResult::total`).
    ///
    /// `limit` and `offset` translate directly to SQL `LIMIT` / `OFFSET`; the caller is
    /// expected to derive them from the public pagination contract as
    /// `limit = size` and `offset = page * size`.
    ///
    /// Each description is looked up by its node's key, one query per node, which costs a page of
    /// primary-key lookups in exchange for reusing the single-node reads. Everything shares one
    /// transaction (no tearing if another writer commits in between).
    pub(crate) async fn get_nym_nodes_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(Vec<NymNode>, i64)> {
        let mut tx = self.connection_pool.begin().await?;

        let bonds = sqlx::query_as!(
            BondedNymNode,
            r#"
            SELECT *
            FROM nym_node_bond
            ORDER BY node_id ASC
            LIMIT ? OFFSET ?
            "#,
            limit,
            offset
        )
        .fetch_all(&mut *tx)
        .await?;

        let mut nodes = Vec::with_capacity(bonds.len());
        for bond in bonds {
            let description = get_node_description(&mut tx, bond.node_id).await?;
            nodes.push(NymNode { bond, description });
        }

        let total = sqlx::query_scalar!("SELECT COUNT(*) FROM nym_node_bond")
            .fetch_one(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok((nodes, total))
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

        let rows = sqlx::query_as!(
            TestRunInProgress,
            r#"
            SELECT node_id, started_at, expires_at, test_kind AS "test_kind: TestKind"
            FROM testrun_in_progress
            ORDER BY started_at ASC
            LIMIT ? OFFSET ?
            "#,
            limit,
            offset
        )
        .fetch_all(&mut *tx)
        .await?;

        let total = sqlx::query_scalar!("SELECT COUNT(*) FROM testrun_in_progress")
            .fetch_one(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok((rows, total))
    }

    /// Deletes every completed run whose `test_timestamp` is older than `cutoff`, from every kind's
    /// results table, returning how many went in total.
    ///
    /// Intended to be called periodically with `now - eviction_age` as the cutoff to keep
    /// the local database from growing unboundedly. Rows that are evicted are assumed to
    /// have already been submitted to the nym-api for persistent storage.
    ///
    /// Driven off the kinds themselves, so a new kind cannot be added without its table being
    /// evicted. Each kind's `last_tested_at` is deliberately left alone, so an evicted result does
    /// not make the node read as never-tested and jump the assignment queue.
    pub(crate) async fn evict_old_testruns(&self, cutoff: OffsetDateTime) -> anyhow::Result<u64> {
        let mut evicted = 0;
        for kind in TestKind::iter() {
            evicted += match kind {
                TestKind::MixnodeLiveness => {
                    self.evict_old_mixnode_liveness_testruns(cutoff).await?
                }
                TestKind::GatewayLiveness => {
                    self.evict_old_gateway_liveness_testruns(cutoff).await?
                }
                TestKind::MixnodeStress => self.evict_old_mixnode_stress_testruns(cutoff).await?,
            };
        }
        Ok(evicted)
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
        match test_kind {
            TestKind::MixnodeLiveness => self.get_mixnode_liveness_testruns_after(after_id).await,
            TestKind::GatewayLiveness => self.get_gateway_liveness_testruns_after(after_id).await,
            TestKind::MixnodeStress => self.get_mixnode_stress_testruns_after(after_id).await,
        }
    }

    /// Every run of `test_kind` stored within `window`, across all nodes, in no particular order.
    pub(crate) async fn get_testruns_in_window(
        &self,
        test_kind: TestKind,
        window: TestRunWindow,
    ) -> anyhow::Result<Vec<CompletedTestRun>> {
        match test_kind {
            TestKind::MixnodeLiveness => self.get_mixnode_liveness_testruns_in_window(window).await,
            TestKind::GatewayLiveness => self.get_gateway_liveness_testruns_in_window(window).await,
            TestKind::MixnodeStress => self.get_mixnode_stress_testruns_in_window(window).await,
        }
    }

    /// Stores aggregates that are not already stored, leaving any that are exactly as they were, in
    /// one transaction.
    ///
    /// Re-materialising an epoch is a no-op rather than a correction, which is what keeps a served
    /// value stable. `ON CONFLICT DO NOTHING` rather than `INSERT OR IGNORE`, which would swallow a
    /// failed CHECK or an unknown node as readily as the duplicate this is meant to tolerate.
    pub(crate) async fn batch_insert_mixnet_epoch_aggregates(
        &self,
        aggregates: &[MixnetEpochAggregate],
    ) -> anyhow::Result<()> {
        let mut tx = self.connection_pool.begin().await?;

        for aggregate in aggregates {
            sqlx::query!(
                r#"
                INSERT INTO mixnet_epoch_aggregate (mixnet_epoch, epoch_start, node_id, test_kind, score, samples)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT (mixnet_epoch, node_id, test_kind) DO NOTHING
                "#,
                aggregate.mixnet_epoch,
                aggregate.epoch_start,
                aggregate.node_id,
                aggregate.test_kind,
                aggregate.score,
                aggregate.samples,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// The newest epoch that has aggregates stored, or `None` when none has.
    ///
    /// An epoch in which nothing was measured leaves no row, so it cannot be seen here and is
    /// recomputed, to the same empty result, after a restart.
    pub(crate) async fn get_last_materialised_mixnet_epoch(&self) -> anyhow::Result<Option<i64>> {
        let last = sqlx::query_scalar!("SELECT MAX(mixnet_epoch) FROM mixnet_epoch_aggregate")
            .fetch_one(&self.connection_pool)
            .await?;
        Ok(last)
    }

    /// Every aggregate stored for `mixnet_epoch`, ordered by node and then kind.
    pub(crate) async fn get_mixnet_epoch_aggregates(
        &self,
        mixnet_epoch: i64,
    ) -> anyhow::Result<Vec<MixnetEpochAggregate>> {
        let aggregates = sqlx::query_as!(
            MixnetEpochAggregate,
            r#"
            SELECT mixnet_epoch, epoch_start, node_id, test_kind AS "test_kind: TestKind", score, samples
            FROM mixnet_epoch_aggregate
            WHERE mixnet_epoch = ?
            ORDER BY node_id, test_kind
            "#,
            mixnet_epoch
        )
        .fetch_all(&self.connection_pool)
        .await?;
        Ok(aggregates)
    }

    /// One node's aggregates for `mixnet_epoch`, one per kind that measured it, ordered by kind.
    pub(crate) async fn get_mixnet_epoch_aggregates_for_node(
        &self,
        mixnet_epoch: i64,
        node_id: i64,
    ) -> anyhow::Result<Vec<MixnetEpochAggregate>> {
        let aggregates = sqlx::query_as!(
            MixnetEpochAggregate,
            r#"
            SELECT mixnet_epoch, epoch_start, node_id, test_kind AS "test_kind: TestKind", score, samples
            FROM mixnet_epoch_aggregate
            WHERE mixnet_epoch = ? AND node_id = ?
            ORDER BY test_kind
            "#,
            mixnet_epoch,
            node_id
        )
        .fetch_all(&self.connection_pool)
        .await?;
        Ok(aggregates)
    }

    /// Deletes every aggregate of an epoch that began before `cutoff`, returning how many went.
    pub(crate) async fn evict_old_mixnet_epoch_aggregates(
        &self,
        cutoff: OffsetDateTime,
    ) -> anyhow::Result<u64> {
        let evicted = sqlx::query!(
            "DELETE FROM mixnet_epoch_aggregate WHERE epoch_start < ?",
            cutoff
        )
        .execute(&self.connection_pool)
        .await?
        .rows_affected();
        Ok(evicted)
    }

    /// Stores the on-chain standing of every node in `capabilities`, replacing what was cached for
    /// it, in one transaction.
    pub(crate) async fn batch_upsert_node_chain_capabilities(
        &self,
        capabilities: &[NodeChainCapability],
    ) -> anyhow::Result<()> {
        let mut tx = self.connection_pool.begin().await?;

        for capability in capabilities {
            sqlx::query!(
                r#"
                INSERT INTO node_chain_capability (
                    node_id,
                    balance,
                    is_feegrant_grantee,
                    refreshed_at,
                    next_refresh_due_at
                ) VALUES (?, ?, ?, ?, ?)
                ON CONFLICT (node_id) DO UPDATE SET
                    balance             = excluded.balance,
                    is_feegrant_grantee = excluded.is_feegrant_grantee,
                    refreshed_at        = excluded.refreshed_at,
                    next_refresh_due_at = excluded.next_refresh_due_at
                "#,
                capability.node_id,
                capability.balance,
                capability.is_feegrant_grantee,
                capability.refreshed_at,
                capability.next_refresh_due_at,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Every cached on-chain standing, ordered by node id.
    #[cfg(test)]
    pub(crate) async fn get_node_chain_capabilities(
        &self,
    ) -> anyhow::Result<Vec<NodeChainCapability>> {
        let capabilities = sqlx::query_as!(
            NodeChainCapability,
            "SELECT * FROM node_chain_capability ORDER BY node_id"
        )
        .fetch_all(&self.connection_pool)
        .await?;
        Ok(capabilities)
    }

    /// Every described node whose on-chain standing is not cached yet or fell due by `now`, ordered
    /// by node id. Driven from the descriptions, so a node that is no longer bonded is never queried.
    pub(crate) async fn get_nodes_awaiting_capability_refresh(
        &self,
        now: OffsetDateTime,
    ) -> anyhow::Result<Vec<NodeAwaitingCapabilityRefresh>> {
        let nodes = sqlx::query_as!(
            NodeAwaitingCapabilityRefresh,
            r#"
            SELECT d.node_id AS "node_id!", d.declared_chain_address
            FROM nym_node_description d
            LEFT JOIN node_chain_capability c ON c.node_id = d.node_id
            WHERE c.node_id IS NULL OR c.next_refresh_due_at <= ?
            ORDER BY d.node_id
            "#,
            now
        )
        .fetch_all(&self.connection_pool)
        .await?;
        Ok(nodes)
    }

    /// Every described node with its cached on-chain standing, if any, ordered by node id.
    pub(crate) async fn get_config_score_candidates(
        &self,
    ) -> anyhow::Result<Vec<ConfigScoreCandidate>> {
        let candidates = sqlx::query_as!(
            ConfigScoreCandidate,
            r#"
            SELECT
                d.node_id AS "node_id!",
                d.reported_version,
                d.accepted_terms_and_conditions,
                c.balance AS "balance?",
                c.is_feegrant_grantee AS "is_feegrant_grantee?"
            FROM nym_node_description d
            LEFT JOIN node_chain_capability c ON c.node_id = d.node_id
            ORDER BY d.node_id
            "#
        )
        .fetch_all(&self.connection_pool)
        .await?;
        Ok(candidates)
    }

    /// Stores config scores that are not already stored, leaving any that are exactly as they were,
    /// in one transaction. As with the aggregates, re-materialising an epoch is a no-op.
    pub(crate) async fn batch_insert_mixnet_epoch_config_scores(
        &self,
        scores: &[MixnetEpochConfigScore],
    ) -> anyhow::Result<()> {
        let mut tx = self.connection_pool.begin().await?;

        for score in scores {
            sqlx::query!(
                r#"
                INSERT INTO mixnet_epoch_config_score (
                    mixnet_epoch,
                    epoch_start,
                    node_id,
                    score,
                    versions_behind,
                    accepted_terms_and_conditions,
                    runs_nym_node_binary,
                    has_sufficient_tokens,
                    is_feegrant_grantee
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (mixnet_epoch, node_id) DO NOTHING
                "#,
                score.mixnet_epoch,
                score.epoch_start,
                score.node_id,
                score.score,
                score.versions_behind,
                score.accepted_terms_and_conditions,
                score.runs_nym_node_binary,
                score.has_sufficient_tokens,
                score.is_feegrant_grantee,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Every config score stored for `mixnet_epoch`, ordered by node.
    pub(crate) async fn get_mixnet_epoch_config_scores(
        &self,
        mixnet_epoch: i64,
    ) -> anyhow::Result<Vec<MixnetEpochConfigScore>> {
        let scores = sqlx::query_as!(
            MixnetEpochConfigScore,
            "SELECT * FROM mixnet_epoch_config_score WHERE mixnet_epoch = ? ORDER BY node_id",
            mixnet_epoch
        )
        .fetch_all(&self.connection_pool)
        .await?;
        Ok(scores)
    }

    /// One node's config score for `mixnet_epoch`, or `None` if none was stored.
    pub(crate) async fn get_mixnet_epoch_config_score_for_node(
        &self,
        mixnet_epoch: i64,
        node_id: i64,
    ) -> anyhow::Result<Option<MixnetEpochConfigScore>> {
        let score = sqlx::query_as!(
            MixnetEpochConfigScore,
            "SELECT * FROM mixnet_epoch_config_score WHERE mixnet_epoch = ? AND node_id = ?",
            mixnet_epoch,
            node_id
        )
        .fetch_optional(&self.connection_pool)
        .await?;
        Ok(score)
    }

    /// Deletes every config score of an epoch that began before `cutoff`, returning how many went.
    pub(crate) async fn evict_old_mixnet_epoch_config_scores(
        &self,
        cutoff: OffsetDateTime,
    ) -> anyhow::Result<u64> {
        let evicted = sqlx::query!(
            "DELETE FROM mixnet_epoch_config_score WHERE epoch_start < ?",
            cutoff
        )
        .execute(&self.connection_pool)
        .await?
        .rows_affected();
        Ok(evicted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::{
        FIXTURE_SEEN_AT, NodeTestState, described_node, gateway, minimal_measurement,
        minimal_measurements, minimal_test_run, mixnode,
    };
    use nym_network_monitor_orchestrator_requests::models::LatencyDistribution;
    use std::net::IpAddr;
    use std::time::Duration;
    use time::macros::datetime;

    async fn setup() -> StorageManager {
        crate::storage::NetworkMonitorStorage::in_memory()
            .await
            .storage_manager
    }

    /// Stores `nodes` as one refresh, at the time every fixture bond carries.
    async fn seed_nodes(db: &StorageManager, nodes: &[NymNode]) {
        db.store_refresh(nodes, FIXTURE_SEEN_AT).await.unwrap()
    }

    /// Seeds one described mixnode, so that runs and assignments referencing `node_id` satisfy the
    /// foreign key and the mixnode role filter.
    async fn seed_node(db: &StorageManager, node_id: i64) {
        seed_nodes(db, &[mixnode(node_id)]).await
    }

    /// Inserts a run of `kind`, every interface at its baseline, and returns its id. No in-flight
    /// lock is needed or released.
    async fn insert_run_of(db: &StorageManager, kind: TestKind, run: &NewTestRun) -> i64 {
        db.insert_test_run(run, &minimal_measurements(kind))
            .await
            .unwrap()
    }

    /// Inserts a `mixnode_stress` run and returns its id.
    async fn insert_run(db: &StorageManager, run: &NewTestRun) -> i64 {
        insert_run_of(db, TestKind::MixnodeStress, run).await
    }

    /// Submits a `mixnode_stress` result for a node dispatched for it, as an agent would, which
    /// stores the run and releases the node's lock.
    async fn submit_run(db: &StorageManager, run: &NewTestRun) {
        let submission = db
            .submit_testrun(run, &minimal_measurements(TestKind::MixnodeStress))
            .await
            .unwrap();
        assert_eq!(submission, TestRunSubmission::Stored);
    }

    /// Reads one kind's work-state row, or `None` if neither the assignment nor a result has
    /// touched it yet.
    async fn work_state(
        db: &StorageManager,
        node_id: i64,
        test_kind: TestKind,
    ) -> Option<NodeTestState> {
        sqlx::query_as!(
            NodeTestState,
            r#"
            SELECT test_kind AS "test_kind: TestKind", last_tested_at, last_tested_ip
            FROM node_test_state
            WHERE node_id = ? AND test_kind = ?
            "#,
            node_id,
            test_kind
        )
        .fetch_optional(&db.connection_pool)
        .await
        .unwrap()
    }

    /// Every work-state row a node holds, ordered by kind so assertions can index them.
    async fn work_states(db: &StorageManager, node_id: i64) -> Vec<NodeTestState> {
        sqlx::query_as!(
            NodeTestState,
            r#"
            SELECT test_kind AS "test_kind: TestKind", last_tested_at, last_tested_ip
            FROM node_test_state
            WHERE node_id = ?
            ORDER BY test_kind
            "#,
            node_id
        )
        .fetch_all(&db.connection_pool)
        .await
        .unwrap()
    }

    // A far-future cutoff that effectively disables the staleness gate,
    // used in tests that are not concerned with that behaviour.
    fn no_staleness_gate() -> OffsetDateTime {
        datetime!(9999-12-31 23:59:59 UTC)
    }

    /// A request for `kind` at `now`, with an hour-long lease and the given staleness gate.
    fn request(
        kind: TestKind,
        now: OffsetDateTime,
        last_tested_before: OffsetDateTime,
        wave_size: usize,
    ) -> AssignmentRequest {
        AssignmentRequest {
            kind,
            now,
            last_tested_before,
            expires_at: now + time::Duration::hours(1),
            wave_size,
        }
    }

    /// A `mixnode_stress` request, i.e. the one-target wave that kind always asks for.
    fn stress_request(
        now: OffsetDateTime,
        last_tested_before: OffsetDateTime,
    ) -> AssignmentRequest {
        request(TestKind::MixnodeStress, now, last_tested_before, 1)
    }

    /// Assigns `mixnode_stress` at `now`, returning the single target such a request can produce.
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

    /// Seeds a kind's rotation pointer, standing in for an earlier assignment of that kind.
    async fn seed_rotation_pointer(
        db: &StorageManager,
        node_id: i64,
        test_kind: TestKind,
        last_tested_ip: &str,
    ) {
        sqlx::query!(
            "INSERT INTO node_test_state (node_id, test_kind, last_tested_ip) VALUES (?, ?, ?)",
            node_id,
            test_kind,
            last_tested_ip
        )
        .execute(&db.connection_pool)
        .await
        .unwrap();
    }

    /// Marks a node as in-progress for `mixnode_stress` with an hour-long lease.
    async fn mark_in_progress(db: &StorageManager, node_id: i64, started_at: OffsetDateTime) {
        db.mark_testrun_in_progress(
            node_id,
            started_at,
            started_at + time::Duration::hours(1),
            TestKind::MixnodeStress,
        )
        .await
        .unwrap()
    }

    /// `node` as a refresh at `seen_at` reads its bond.
    fn seen_at(node: NymNode, seen_at: OffsetDateTime) -> NymNode {
        NymNode {
            bond: BondedNymNode {
                last_seen_bonded: seen_at,
                ..node.bond
            },
            description: node.description,
        }
    }

    /// `node`'s bond alone, i.e. what a refresh stores for a node whose describe failed.
    fn bond_only(node: NymNode) -> NymNode {
        NymNode {
            bond: node.bond,
            description: None,
        }
    }

    fn us(micros: u64) -> Duration {
        Duration::from_micros(micros)
    }

    /// A measurement whose figures all differ, offset by `base`, so a value written to or read from
    /// the wrong column shows up. Microsecond values, since that is the stored precision.
    fn distinct_measurement(base: u64, received_duplicates: bool) -> InterfaceMeasurement {
        InterfaceMeasurement {
            ingress_noise_handshake: Some(us(base + 1)),
            egress_noise_handshake: Some(us(base + 2)),
            sphinx_packet_delay: us(base + 3),
            packets_sent: base as usize + 4,
            packets_received: base as usize + 5,
            approximate_latency: Some(us(base + 6)),
            packets_statistics: Some(LatencyDistribution {
                minimum: us(base + 7),
                mean: us(base + 8),
                median: us(base + 9),
                maximum: us(base + 10),
                standard_deviation: us(base + 11),
            }),
            received_duplicates,
        }
    }

    mod store_refresh {
        use super::*;

        async fn bond_count(db: &StorageManager) -> i64 {
            sqlx::query_scalar!("SELECT COUNT(*) FROM nym_node_bond")
                .fetch_one(&db.connection_pool)
                .await
                .unwrap()
        }

        async fn description_count(db: &StorageManager) -> i64 {
            sqlx::query_scalar!("SELECT COUNT(*) FROM nym_node_description")
                .fetch_one(&db.connection_pool)
                .await
                .unwrap()
        }

        #[tokio::test]
        async fn stores_every_bond_and_description() {
            let db = setup().await;
            seed_nodes(&db, &[mixnode(1), mixnode(2), gateway(3)]).await;

            assert_eq!(bond_count(&db).await, 3);
            assert_eq!(description_count(&db).await, 3);
        }

        #[tokio::test]
        async fn a_later_description_replaces_the_previous_one_whole() {
            let db = setup().await;
            seed_node(&db, 1).await;

            // the same node, now announcing another address and taking on the gateway role
            let updated = described_node(1, "9.9.9.9", true, true);
            seed_nodes(&db, &[updated, mixnode(2)]).await;

            let description = db
                .get_nym_node_by_id(1)
                .await
                .unwrap()
                .unwrap()
                .description
                .unwrap();
            assert_eq!(description.announced_ips, "9.9.9.9");
            assert!(description.mixnode_enabled);
            assert!(description.gateway_enabled);
            assert_eq!(description.clients_ws_port, Some(9000));

            assert_eq!(bond_count(&db).await, 2);
        }

        // A failed describe must not cost the node what an earlier cycle learned: losing the
        // description makes it ineligible for every kind, which would drop a node that is merely
        // slow out of testing until a later cycle answered.
        #[tokio::test]
        async fn a_bond_only_refresh_keeps_what_was_already_learned() {
            let db = setup().await;
            let described = described_node(1, "1.2.3.4", true, true);
            let learned = described.description.clone();
            seed_nodes(&db, &[described]).await;

            let later = datetime!(2025-06-02 00:00:00 UTC);
            db.store_refresh(&[seen_at(bond_only(mixnode(1)), later)], later)
                .await
                .unwrap();

            let node = db.get_nym_node_by_id(1).await.unwrap().unwrap();
            let kept = node.description.unwrap();
            let learned = learned.unwrap();
            assert_eq!(kept.announced_ips, learned.announced_ips);
            assert_eq!(kept.noise_key, learned.noise_key);
            assert_eq!(kept.sphinx_key, learned.sphinx_key);
            assert_eq!(kept.key_rotation_id, learned.key_rotation_id);
            assert!(kept.mixnode_enabled);
            assert!(kept.gateway_enabled);
            assert_eq!(kept.clients_ws_port, Some(9000));

            // the one thing it does record is that the bond is still there
            assert_eq!(node.bond.last_seen_bonded, later);
        }

        // A node seen for the first time still exists, as a bond with no description: the one state
        // that genuinely means "never described" rather than "described once".
        #[tokio::test]
        async fn a_bond_only_refresh_inserts_an_undescribed_node() {
            let db = setup().await;
            seed_nodes(&db, &[bond_only(mixnode(7))]).await;

            let node = db.get_nym_node_by_id(7).await.unwrap().unwrap();
            assert_eq!(node.bond.identity_key, mixnode(7).bond.identity_key);
            assert!(node.description.is_none());
        }

        // and being described afterwards fills it in, so a node that answers late is not stuck as a
        // stub
        #[tokio::test]
        async fn a_later_describe_fills_in_a_bond_only_node() {
            let db = setup().await;
            seed_nodes(&db, &[bond_only(mixnode(1))]).await;
            seed_node(&db, 1).await;

            let node = db.get_nym_node_by_id(1).await.unwrap().unwrap();
            assert!(node.description.unwrap().mixnode_enabled);
        }

        #[tokio::test]
        async fn an_empty_refresh_writes_nothing() {
            let db = setup().await;
            seed_nodes(&db, &[]).await;

            assert_eq!(bond_count(&db).await, 0);
        }

        // An unbonded node is one a successful contract read no longer lists, so its bond carries an
        // older refresh time than this one's. It must stop being assigned, which losing its
        // description does, while its bond stays for the read surface.
        #[tokio::test]
        async fn a_node_missing_from_a_refresh_loses_its_description_but_keeps_its_bond() {
            let db = setup().await;
            seed_nodes(&db, &[mixnode(1), mixnode(2)]).await;

            let later = datetime!(2025-06-02 00:00:00 UTC);
            db.store_refresh(&[seen_at(mixnode(1), later)], later)
                .await
                .unwrap();

            let still_bonded = db.get_nym_node_by_id(1).await.unwrap().unwrap();
            assert!(still_bonded.description.is_some());

            let unbonded = db.get_nym_node_by_id(2).await.unwrap().unwrap();
            assert!(unbonded.description.is_none());
            assert_eq!(unbonded.bond.last_seen_bonded, FIXTURE_SEEN_AT);

            // and it is no longer handed out: a wave with room for both takes only the bonded node
            let wave = db
                .assign_next_testruns(&request(
                    TestKind::MixnodeStress,
                    later,
                    no_staleness_gate(),
                    10,
                ))
                .await
                .unwrap();
            let assigned: Vec<_> = wave.iter().map(|target| target.node.node_id).collect();
            assert_eq!(assigned, vec![1]);
        }

        // A gateway liveness probe cannot open a session without the client websocket port, so a
        // gateway-capable description without it must be unstorable, and a port on a node that is
        // not gateway-capable is equally a malformed reading. A rejected refresh writes nothing.
        #[tokio::test]
        async fn a_description_whose_port_disagrees_with_its_gateway_role_is_rejected() {
            let portless_gateway = {
                let mut node = gateway(1);
                if let Some(description) = node.description.as_mut() {
                    description.clients_ws_port = None;
                }
                node
            };
            let mixnode_with_port = {
                let mut node = mixnode(1);
                if let Some(description) = node.description.as_mut() {
                    description.clients_ws_port = Some(9000);
                }
                node
            };

            for malformed in [portless_gateway, mixnode_with_port] {
                let db = setup().await;
                let result = db.store_refresh(&[malformed], FIXTURE_SEEN_AT).await;

                assert!(result.is_err());
                assert_eq!(bond_count(&db).await, 0);
            }
        }

        // the config score is computed from these as stored, so an input swapped or left out of the
        // replacing write would score the node on something it never reported
        #[tokio::test]
        async fn a_later_description_replaces_the_config_score_inputs() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let other_address = mixnode(2).description.unwrap().declared_chain_address;
            let mut updated = mixnode(1);
            let description = updated.description.as_mut().unwrap();
            description.reported_version = "1.2.3".to_string();
            description.binary_name = "not-nym-node".to_string();
            description.accepted_terms_and_conditions = false;
            description.declared_chain_address = other_address.clone();
            seed_nodes(&db, &[updated]).await;

            let stored = db
                .get_nym_node_by_id(1)
                .await
                .unwrap()
                .unwrap()
                .description
                .unwrap();
            assert_eq!(stored.reported_version, "1.2.3");
            assert_eq!(stored.binary_name, "not-nym-node");
            assert!(!stored.accepted_terms_and_conditions);
            assert_eq!(stored.declared_chain_address, other_address);
        }
    }

    mod record_testrun {
        use super::*;

        #[tokio::test]
        async fn persists_run_level_fields() {
            let db = setup().await;
            seed_node(&db, 1).await;
            let mut run = minimal_test_run(1);
            run.time_taken_us = 1234;
            run.error = Some("timeout".to_string());
            let id = insert_run(&db, &run).await;

            let stored = db
                .get_testrun_by_id(TestKind::MixnodeStress, id)
                .await
                .unwrap()
                .unwrap()
                .run;
            assert_eq!(stored.node_id, 1);
            assert_eq!(stored.tested_address, "1.2.3.4:1789");
            assert_eq!(stored.test_timestamp, run.test_timestamp);
            assert_eq!(stored.time_taken_us, 1234);
            assert_eq!(stored.error.as_deref(), Some("timeout"));
        }

        // Each kind is written through its own hand-written INSERT, whose binds are positional, so a
        // swapped pair of same-typed figures would compile and silently store the wrong values.
        // Every figure here is distinct, and two interfaces differ in every figure, so any swap -
        // within a group or between a gateway's two - changes what reads back.
        #[tokio::test]
        async fn every_figure_of_every_kind_reads_back_as_written() {
            let db = setup().await;
            seed_node(&db, 1).await;

            for measurements in [
                RunMeasurements::MixnodeLiveness {
                    mix_forwarding: distinct_measurement(100, true),
                },
                RunMeasurements::GatewayLiveness {
                    client_ingest: distinct_measurement(200, true),
                    client_delivery: distinct_measurement(300, false),
                },
                RunMeasurements::MixnodeStress {
                    mix_forwarding: distinct_measurement(400, true),
                },
            ] {
                let kind = TestKind::from(measurements.kind());
                let id = db
                    .insert_test_run(&minimal_test_run(1), &measurements)
                    .await
                    .unwrap();

                let stored = db.get_testrun_by_id(kind, id).await.unwrap().unwrap();
                assert_eq!(stored.measurements, measurements);
            }
        }

        #[tokio::test]
        async fn records_the_kinds_work_state() {
            let db = setup().await;
            seed_node(&db, 1).await;
            let run = minimal_test_run(1);
            insert_run(&db, &run).await;

            let state = work_state(&db, 1, TestKind::MixnodeStress).await.unwrap();
            assert_eq!(state.last_tested_at, Some(run.test_timestamp));
        }

        // each kind keeps its own staleness position, so recording a run under one must not touch
        // another's
        #[tokio::test]
        async fn one_kinds_result_leaves_the_others_untouched() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let stress = minimal_test_run(1);
            insert_run(&db, &stress).await;

            assert_eq!(work_states(&db, 1).await.len(), 1);

            let mut liveness = minimal_test_run(1);
            liveness.test_timestamp = datetime!(2025-06-02 12:00:00 UTC);
            insert_run_of(&db, TestKind::MixnodeLiveness, &liveness).await;

            let states = work_states(&db, 1).await;
            assert_eq!(states.len(), 2);
            // each kind carries the timestamp of its own run, not of the other's
            assert_eq!(states[0].test_kind, TestKind::MixnodeLiveness);
            assert_eq!(states[0].last_tested_at, Some(liveness.test_timestamp));
            assert_eq!(states[1].test_kind, TestKind::MixnodeStress);
            assert_eq!(states[1].last_tested_at, Some(stress.test_timestamp));
        }

        #[tokio::test]
        async fn a_submitted_result_is_stored_and_releases_the_nodes_lock() {
            let db = setup().await;
            seed_node(&db, 1).await;
            mark_in_progress(&db, 1, datetime!(2025-06-01 11:00:00 UTC)).await;

            let submission = db
                .submit_testrun(
                    &minimal_test_run(1),
                    &minimal_measurements(TestKind::MixnodeStress),
                )
                .await
                .unwrap();

            assert_eq!(submission, TestRunSubmission::Stored);
            assert!(db.get_testrun_in_progress(1).await.unwrap().is_none());
            let stored = db
                .get_testruns_after(TestKind::MixnodeStress, 0)
                .await
                .unwrap();
            assert_eq!(stored.len(), 1);
        }

        // a node without an in-flight row was freed by the lease sweep, so the result can no
        // longer be attributed to the dispatch it answers
        #[tokio::test]
        async fn a_result_for_a_node_with_no_lock_stores_nothing() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let submission = db
                .submit_testrun(
                    &minimal_test_run(1),
                    &minimal_measurements(TestKind::MixnodeStress),
                )
                .await
                .unwrap();

            assert_eq!(submission, TestRunSubmission::LeaseExpired);
            let stored = db
                .get_testruns_after(TestKind::MixnodeStress, 0)
                .await
                .unwrap();
            assert!(stored.is_empty());
            assert!(work_state(&db, 1, TestKind::MixnodeStress).await.is_none());
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
            lease_of(db, node_id, TestKind::MixnodeStress, started_at, expires_at).await
        }

        /// The same, under a chosen kind.
        async fn lease_of(
            db: &StorageManager,
            node_id: i64,
            test_kind: TestKind,
            started_at: OffsetDateTime,
            expires_at: OffsetDateTime,
        ) {
            db.mark_testrun_in_progress(node_id, started_at, expires_at, test_kind)
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
            assert_eq!(cleared.get(&TestKind::MixnodeStress).copied(), Some(1));
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

            lease_of(&db, 1, TestKind::MixnodeStress, expired_at, expired_at).await;
            lease_of(&db, 2, TestKind::MixnodeLiveness, expired_at, expired_at).await;
            lease_of(&db, 3, TestKind::MixnodeLiveness, expired_at, expired_at).await;

            let cleared = db
                .clear_expired_testruns_in_progress(datetime!(2025-06-01 12:00:00 UTC))
                .await
                .unwrap();

            assert_eq!(cleared.get(&TestKind::MixnodeStress).copied(), Some(1));
            assert_eq!(cleared.get(&TestKind::MixnodeLiveness).copied(), Some(2));
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

            lease_of(&db, 1, TestKind::MixnodeLiveness, started_at, expires_at).await;
            lease_of(&db, 2, TestKind::MixnodeLiveness, started_at, expires_at).await;

            let counts = db.count_testruns_in_progress_by_kind().await.unwrap();
            assert_eq!(counts.get(&TestKind::MixnodeLiveness).copied(), Some(2));
            assert_eq!(counts.get(&TestKind::MixnodeStress), None);
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

            let kind = TestKind::MixnodeStress;
            assert!(db.get_testrun_by_id(kind, old_id).await.unwrap().is_none());
            assert!(
                db.get_testrun_by_id(kind, recent_id)
                    .await
                    .unwrap()
                    .is_some()
            );
        }

        #[tokio::test]
        async fn preserves_runs_at_or_after_cutoff() {
            let db = setup().await;
            seed_node(&db, 1).await;
            let mut run = minimal_test_run(1);
            run.test_timestamp = datetime!(2025-03-01 00:00:00 UTC);
            let id = insert_run(&db, &run).await;

            // cutoff is exactly at the run's timestamp - should NOT be evicted (strict <)
            db.evict_old_testruns(datetime!(2025-03-01 00:00:00 UTC))
                .await
                .unwrap();

            assert!(
                db.get_testrun_by_id(TestKind::MixnodeStress, id)
                    .await
                    .unwrap()
                    .is_some()
            );
        }

        // every kind keeps its results in a table of its own, so the sweep has to reach each of
        // them: a table it missed would grow without bound
        #[tokio::test]
        async fn evicts_from_every_kinds_table() {
            let db = setup().await;
            seed_nodes(&db, &[described_node(1, "1.2.3.4", true, true)]).await;

            let old_run = NewTestRun {
                test_timestamp: datetime!(2025-01-01 00:00:00 UTC),
                ..minimal_test_run(1)
            };
            let mut old_ids = Vec::new();
            for kind in TestKind::iter() {
                old_ids.push((kind, insert_run_of(&db, kind, &old_run).await));
            }
            let recent_id = insert_run(&db, &minimal_test_run(1)).await;

            let evicted = db
                .evict_old_testruns(datetime!(2025-03-01 00:00:00 UTC))
                .await
                .unwrap();
            assert_eq!(evicted, old_ids.len() as u64);

            for (kind, id) in old_ids {
                assert!(db.get_testrun_by_id(kind, id).await.unwrap().is_none());
            }
            assert!(
                db.get_testrun_by_id(TestKind::MixnodeStress, recent_id)
                    .await
                    .unwrap()
                    .is_some()
            );
        }

        #[tokio::test]
        async fn does_nothing_when_no_old_runs() {
            let db = setup().await;
            seed_node(&db, 1).await;
            insert_run(&db, &minimal_test_run(1)).await;

            // cutoff is well in the past - nothing should be evicted
            let evicted = db
                .evict_old_testruns(datetime!(2000-01-01 00:00:00 UTC))
                .await
                .unwrap();
            assert_eq!(evicted, 0);

            let (_, total) = db
                .get_testruns_paginated(TestKind::MixnodeStress, 10, 0)
                .await
                .unwrap();
            assert_eq!(total, 1);
        }
    }

    mod assign_next_testruns {
        use super::*;

        // One assignment, many targets, each locked and rotated in its own right - the property that
        // separates a wave from a single-target assignment.
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
                    ..request(TestKind::MixnodeLiveness, now, no_staleness_gate(), 2)
                })
                .await
                .unwrap();

            assert_eq!(wave.len(), 2);

            for target in &wave {
                let node_id = target.node.node_id;

                // a lock per target, each carrying this wave's lease rather than one shared row
                let row = db.get_testrun_in_progress(node_id).await.unwrap().unwrap();
                assert_eq!(row.expires_at, now + time::Duration::minutes(1));
                assert_eq!(row.test_kind, TestKind::MixnodeLiveness);

                // and a rotation pointer per target, under the kind that was dispatched
                let state = work_state(&db, node_id, TestKind::MixnodeLiveness)
                    .await
                    .unwrap();
                assert_eq!(state.last_tested_ip.as_deref(), Some("1.2.3.4"));
            }

            // the target the cap left behind is still assignable, i.e. it was passed over rather
            // than locked
            let remaining = db
                .assign_next_testruns(&request(
                    TestKind::MixnodeLiveness,
                    now,
                    no_staleness_gate(),
                    2,
                ))
                .await
                .unwrap();
            assert_eq!(remaining.len(), 1);
        }

        // each kind probes one role and takes only the nodes reporting it: a gateway request skips
        // a mixnode, a mixnode request skips a gateway, and a dual-role node is a candidate for both
        #[tokio::test]
        async fn each_kind_takes_only_the_nodes_reporting_its_role() {
            let now = datetime!(2025-06-01 12:00:00 UTC);

            for (kind, expected) in [
                (TestKind::GatewayLiveness, vec![1, 3]),
                (TestKind::MixnodeLiveness, vec![2, 3]),
                (TestKind::MixnodeStress, vec![2, 3]),
            ] {
                let db = setup().await;
                seed_nodes(
                    &db,
                    &[
                        gateway(1),
                        mixnode(2),
                        described_node(3, "1.2.3.4", true, true),
                    ],
                )
                .await;

                let wave = db
                    .assign_next_testruns(&request(kind, now, no_staleness_gate(), 10))
                    .await
                    .unwrap();
                let mut assigned: Vec<_> = wave.iter().map(|target| target.node.node_id).collect();
                assigned.sort();
                assert_eq!(assigned, expected, "{kind} took the wrong nodes");

                // the lock records the kind that was dispatched
                let row = db
                    .get_testrun_in_progress(expected[0])
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(row.test_kind, kind);
            }
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
        async fn inserts_in_progress_row_carrying_the_lease_and_kind() {
            let db = setup().await;
            seed_node(&db, 1).await;
            let assigned =
                assign(&db, datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate()).await;
            assert!(assigned.is_some());

            let row = db.get_testrun_in_progress(1).await.unwrap().unwrap();
            assert_eq!(row.started_at, datetime!(2025-06-01 12:00:00 UTC));
            assert_eq!(row.expires_at, datetime!(2025-06-01 13:00:00 UTC));
            assert_eq!(row.test_kind, TestKind::MixnodeStress);
        }

        #[tokio::test]
        async fn advances_the_kinds_rotation_pointer_on_handout() {
            let db = setup().await;
            seed_node(&db, 1).await;
            assign(&db, datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate())
                .await
                .unwrap();

            let state = work_state(&db, 1, TestKind::MixnodeStress).await.unwrap();
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
            assert_eq!(assigned.node.node_id, 2);
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
            assert_eq!(assigned.node.node_id, 1);
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
            assert_eq!(assigned.node.node_id, 2);
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
                TestKind::MixnodeLiveness,
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
                    TestKind::MixnodeLiveness,
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
        // were consulted for liveness, so this also pins that each kind reads only its own.
        #[tokio::test]
        async fn a_node_freed_by_one_kind_is_immediately_assignable_by_another() {
            let db = setup().await;
            seed_node(&db, 1).await;
            let now = datetime!(2025-06-01 12:00:00 UTC);

            // a stress run takes the node, completes, and releases the lock
            assert!(assign(&db, now, no_staleness_gate()).await.is_some());
            submit_run(
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
                    TestKind::MixnodeLiveness,
                    now,
                    now - time::Duration::hours(1),
                    10,
                ))
                .await
                .unwrap();

            assert_eq!(wave.len(), 1);
            assert_eq!(wave[0].node.node_id, 1);
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

        // a run recorded under a different kind must not gate this one: staleness is per kind,
        // which is what keeps a dual-role node eligible for both liveness probes
        #[tokio::test]
        async fn another_kinds_recent_run_does_not_gate_this_one() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let mut liveness = minimal_test_run(1);
            liveness.test_timestamp = datetime!(2025-06-01 12:00:00 UTC);
            insert_run_of(&db, TestKind::MixnodeLiveness, &liveness).await;

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
            assert_eq!(assigned.node.node_id, 2);
        }
    }

    /// The candidate the peek reports, which the scheduler turns into a kind's due time.
    mod peek_next_candidate {
        use super::*;

        #[tokio::test]
        async fn a_kind_with_nothing_eligible_has_no_candidate() {
            let db = setup().await;
            let head = db
                .peek_next_candidate(TestKind::MixnodeLiveness, no_staleness_gate())
                .await
                .unwrap();
            assert!(head.is_none());
        }

        #[tokio::test]
        async fn a_never_tested_node_reads_as_never_tested() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let head = db
                .peek_next_candidate(TestKind::MixnodeLiveness, no_staleness_gate())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(head.node_id, 1);
            assert!(head.last_tested_at.is_none());
        }

        #[tokio::test]
        async fn a_measured_node_reads_as_its_last_run() {
            let db = setup().await;
            seed_node(&db, 1).await;
            insert_run_of(
                &db,
                TestKind::MixnodeLiveness,
                &NewTestRun {
                    test_timestamp: datetime!(2025-06-01 09:00:00 UTC),
                    ..minimal_test_run(1)
                },
            )
            .await;

            let head = db
                .peek_next_candidate(TestKind::MixnodeLiveness, no_staleness_gate())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                head.last_tested_at,
                Some(datetime!(2025-06-01 09:00:00 UTC))
            );

            // the same node under a kind that has never measured it still reads as never tested,
            // so one kind's progress cannot answer for another's
            let other = db
                .peek_next_candidate(TestKind::MixnodeStress, no_staleness_gate())
                .await
                .unwrap()
                .unwrap();
            assert!(other.last_tested_at.is_none());
        }
    }

    mod per_kind_work_state {
        use super::*;

        // the rotation pointer is per kind, so one kind walking the node's address set must leave
        // the others' positions where they were, and must not continue from them either. The decoys
        // are the other two kinds, each already part-way through the same set
        #[tokio::test]
        async fn one_kind_walking_the_address_set_leaves_the_others_pointers_alone() {
            let db = setup().await;
            seed_nodes(&db, &[described_node(1, "1.2.3.4,5.6.7.8", true, true)]).await;

            for decoy in [TestKind::MixnodeLiveness, TestKind::GatewayLiveness] {
                seed_rotation_pointer(&db, 1, decoy, "1.2.3.4").await;
            }

            // the assigned kind has no pointer of its own yet, so it starts at the beginning of the
            // set rather than continuing from where either decoy had got to
            let first = assign(&db, datetime!(2025-06-01 12:00:00 UTC), no_staleness_gate())
                .await
                .unwrap();
            assert_eq!(first.tested_ip, "1.2.3.4".parse::<IpAddr>().unwrap());

            // submit the run, which releases the node's lock and moves its staleness position
            submit_run(&db, &minimal_test_run(1)).await;

            let second = assign(
                &db,
                datetime!(2025-06-01 13:00:00 UTC),
                datetime!(2025-06-01 12:30:00 UTC),
            )
            .await
            .unwrap();
            assert_eq!(second.tested_ip, "5.6.7.8".parse::<IpAddr>().unwrap());

            let assigned = work_state(&db, 1, TestKind::MixnodeStress).await.unwrap();
            assert_eq!(assigned.last_tested_ip.as_deref(), Some("5.6.7.8"));

            // neither decoy moved, so the upsert wrote only its own kind's row
            for decoy in [TestKind::MixnodeLiveness, TestKind::GatewayLiveness] {
                let decoy = work_state(&db, 1, decoy).await.unwrap();
                assert_eq!(decoy.last_tested_ip.as_deref(), Some("1.2.3.4"));
            }
        }

        // the defect that storing `last_tested_at` fixes: read through a join onto the last run, an
        // evicted result made the node read as never-tested, so it jumped the assignment queue
        // ahead of nodes that genuinely had not been measured
        #[tokio::test]
        async fn evicting_a_result_leaves_the_kinds_staleness_position_intact() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let run = minimal_test_run(1);
            let run_id = insert_run(&db, &run).await;

            db.evict_old_testruns(datetime!(2025-06-02 00:00:00 UTC))
                .await
                .unwrap();
            assert!(
                db.get_testrun_by_id(TestKind::MixnodeStress, run_id)
                    .await
                    .unwrap()
                    .is_none()
            );

            // the staleness position the run established survives it, which is the whole reason
            // that timestamp is stored rather than joined
            let state = work_state(&db, 1, TestKind::MixnodeStress).await.unwrap();
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
        async fn returns_the_right_row_when_multiple_exist() {
            let db = setup().await;
            seed_node(&db, 1).await;
            insert_run(&db, &minimal_test_run(1)).await;

            let measurements = RunMeasurements::MixnodeStress {
                mix_forwarding: InterfaceMeasurement {
                    packets_sent: 7,
                    ..minimal_measurement()
                },
            };
            let target_id = db
                .insert_test_run(&minimal_test_run(1), &measurements)
                .await
                .unwrap();
            insert_run(&db, &minimal_test_run(1)).await;

            let fetched = db
                .get_testrun_by_id(TestKind::MixnodeStress, target_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(fetched.id, target_id);
            assert_eq!(fetched.measurements, measurements);
        }
    }

    mod get_latest_testrun_for_node {
        use super::*;

        #[tokio::test]
        async fn returns_none_when_never_tested() {
            let db = setup().await;
            seed_node(&db, 1).await;
            assert!(
                db.get_latest_testrun_for_node(TestKind::MixnodeStress, 1)
                    .await
                    .unwrap()
                    .is_none()
            );
        }

        #[tokio::test]
        async fn returns_the_newest_run_of_the_requested_kind() {
            let db = setup().await;
            seed_node(&db, 1).await;
            seed_node(&db, 2).await;

            let mut older = minimal_test_run(1);
            older.test_timestamp = datetime!(2025-06-01 10:00:00 UTC);
            insert_run(&db, &older).await;

            let mut newest = minimal_test_run(1);
            newest.test_timestamp = datetime!(2025-06-01 12:00:00 UTC);
            let newest_id = insert_run(&db, &newest).await;

            // a newer run of another kind, and another node's newer run, must not be picked up
            let mut other_kind = minimal_test_run(1);
            other_kind.test_timestamp = datetime!(2025-06-01 13:00:00 UTC);
            insert_run_of(&db, TestKind::MixnodeLiveness, &other_kind).await;

            let mut other_node = minimal_test_run(2);
            other_node.test_timestamp = datetime!(2025-06-01 14:00:00 UTC);
            insert_run(&db, &other_node).await;

            let fetched = db
                .get_latest_testrun_for_node(TestKind::MixnodeStress, 1)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(fetched.id, newest_id);
            assert_eq!(fetched.run.test_timestamp, newest.test_timestamp);
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
        async fn returns_inserted_node_with_its_description() {
            let db = setup().await;
            seed_nodes(&db, &[gateway(42)]).await;

            let fetched = db.get_nym_node_by_id(42).await.unwrap().unwrap();
            assert_eq!(fetched.bond.node_id, 42);
            assert_eq!(fetched.bond.identity_key, gateway(42).bond.identity_key);

            let description = fetched.description.unwrap();
            assert_eq!(description.announced_ips, "1.2.3.4");
            assert!(description.gateway_enabled);
            assert_eq!(description.clients_ws_port, Some(9000));
        }
    }

    mod get_testruns_in_progress_paginated {
        use super::*;

        #[tokio::test]
        async fn ordering_is_started_at_ascending() {
            let db = setup().await;
            seed_nodes(&db, &[mixnode(1), mixnode(2), mixnode(3)]).await;

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
            seed_nodes(&db, &[mixnode(1), mixnode(2), mixnode(3)]).await;

            mark_in_progress(&db, 1, datetime!(2025-06-01 10:00:00 UTC)).await;
            mark_in_progress(&db, 2, datetime!(2025-06-01 11:00:00 UTC)).await;
            mark_in_progress(&db, 3, datetime!(2025-06-01 12:00:00 UTC)).await;

            let (rows, total) = db.get_testruns_in_progress_paginated(2, 0).await.unwrap();
            assert_eq!(total, 3);
            let ordered_node_ids: Vec<i64> = rows.iter().map(|r| r.node_id).collect();
            assert_eq!(ordered_node_ids, vec![1, 2]);
        }
    }

    mod get_nym_nodes_paginated {
        use super::*;

        #[tokio::test]
        async fn returns_first_page_and_correct_total() {
            let db = setup().await;
            let nodes: Vec<NymNode> = (1..=5).map(mixnode).collect();
            seed_nodes(&db, &nodes).await;

            let (rows, total) = db.get_nym_nodes_paginated(2, 0).await.unwrap();
            assert_eq!(total, 5);
            let ids: Vec<i64> = rows.iter().map(|r| r.bond.node_id).collect();
            assert_eq!(ids, vec![1, 2]);
        }

        #[tokio::test]
        async fn ordering_is_node_id_ascending() {
            let db = setup().await;
            // insert in non-ascending order to confirm ORDER BY actually sorts
            seed_nodes(&db, &[mixnode(3), mixnode(1), mixnode(2)]).await;

            let (rows, _) = db.get_nym_nodes_paginated(10, 0).await.unwrap();
            let ids: Vec<i64> = rows.iter().map(|r| r.bond.node_id).collect();
            assert_eq!(ids, vec![1, 2, 3]);
        }

        // each description is looked up by its own node's key after the page of bonds is read, so
        // pin that every node comes back with its own, and an undescribed one with none
        #[tokio::test]
        async fn every_node_in_a_page_carries_its_own_description() {
            let db = setup().await;
            seed_nodes(
                &db,
                &[
                    described_node(1, "1.1.1.1", true, false),
                    described_node(2, "2.2.2.2", false, true),
                    bond_only(mixnode(3)),
                ],
            )
            .await;

            let (rows, _) = db.get_nym_nodes_paginated(10, 0).await.unwrap();
            let announced: Vec<_> = rows
                .iter()
                .map(|node| {
                    node.description
                        .as_ref()
                        .map(|description| description.announced_ips.as_str())
                })
                .collect();
            assert_eq!(announced, vec![Some("1.1.1.1"), Some("2.2.2.2"), None]);
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
        async fn ordering_is_test_timestamp_descending() {
            let db = setup().await;
            seed_node(&db, 1).await;
            // insert in mixed order; ensure query returns newest first
            insert_run_at(&db, 1, datetime!(2025-03-01 00:00:00 UTC)).await;
            insert_run_at(&db, 1, datetime!(2025-01-01 00:00:00 UTC)).await;
            insert_run_at(&db, 1, datetime!(2025-02-01 00:00:00 UTC)).await;

            let (rows, total) = db
                .get_testruns_paginated(TestKind::MixnodeStress, 10, 0)
                .await
                .unwrap();
            assert_eq!(total, 3);
            let timestamps: Vec<OffsetDateTime> =
                rows.iter().map(|r| r.run.test_timestamp).collect();
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

            let (rows, total) = db
                .get_testruns_paginated(TestKind::MixnodeStress, 2, 1)
                .await
                .unwrap();
            assert_eq!(total, 3);
            let timestamps: Vec<OffsetDateTime> =
                rows.iter().map(|r| r.run.test_timestamp).collect();
            assert_eq!(
                timestamps,
                vec![
                    datetime!(2025-02-01 00:00:00 UTC),
                    datetime!(2025-01-01 00:00:00 UTC),
                ]
            );
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
        async fn returns_only_runs_for_requested_node() {
            let db = setup().await;
            seed_node(&db, 1).await;
            seed_node(&db, 2).await;

            insert_run(&db, &minimal_test_run(1)).await;
            insert_run(&db, &minimal_test_run(1)).await;
            insert_run(&db, &minimal_test_run(2)).await;

            let (rows, total) = db
                .get_testruns_for_node_paginated(TestKind::MixnodeStress, 1, 50, 0)
                .await
                .unwrap();
            assert_eq!(total, 2);
            assert_eq!(rows.len(), 2);
            assert!(rows.iter().all(|r| r.run.node_id == 1));

            let (rows, total) = db
                .get_testruns_for_node_paginated(TestKind::MixnodeStress, 2, 50, 0)
                .await
                .unwrap();
            assert_eq!(total, 1);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].run.node_id, 2);
        }

        #[tokio::test]
        async fn ordering_is_test_timestamp_descending() {
            let db = setup().await;
            seed_node(&db, 1).await;

            insert_run_at(&db, 1, datetime!(2025-02-01 00:00:00 UTC)).await;
            insert_run_at(&db, 1, datetime!(2025-03-01 00:00:00 UTC)).await;
            insert_run_at(&db, 1, datetime!(2025-01-01 00:00:00 UTC)).await;

            let (rows, _) = db
                .get_testruns_for_node_paginated(TestKind::MixnodeStress, 1, 10, 0)
                .await
                .unwrap();
            let timestamps: Vec<OffsetDateTime> =
                rows.iter().map(|r| r.run.test_timestamp).collect();
            assert_eq!(
                timestamps,
                vec![
                    datetime!(2025-03-01 00:00:00 UTC),
                    datetime!(2025-02-01 00:00:00 UTC),
                    datetime!(2025-01-01 00:00:00 UTC),
                ]
            );
        }
    }

    mod submission_watermark {
        use super::*;

        #[tokio::test]
        async fn absent_until_a_batch_is_submitted() {
            let db = setup().await;
            assert!(
                db.get_last_submitted_testrun_id(TestKind::MixnodeStress)
                    .await
                    .unwrap()
                    .is_none()
            );
        }

        #[tokio::test]
        async fn round_trips_and_overwrites() {
            let db = setup().await;
            db.set_last_submitted_testrun_id(TestKind::MixnodeStress, 7)
                .await
                .unwrap();
            assert_eq!(
                db.get_last_submitted_testrun_id(TestKind::MixnodeStress)
                    .await
                    .unwrap(),
                Some(7)
            );

            db.set_last_submitted_testrun_id(TestKind::MixnodeStress, 9)
                .await
                .unwrap();
            assert_eq!(
                db.get_last_submitted_testrun_id(TestKind::MixnodeStress)
                    .await
                    .unwrap(),
                Some(9)
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

            let pending = db
                .get_testruns_after(TestKind::MixnodeStress, 0)
                .await
                .unwrap();
            assert_eq!(pending.len(), 2);
            assert!(
                pending.iter().all(|run| {
                    matches!(run.measurements, RunMeasurements::MixnodeStress { .. })
                })
            );
        }

        #[tokio::test]
        async fn skips_rows_at_or_below_the_watermark() {
            let db = setup().await;
            seed_node(&db, 1).await;
            let first = insert_run(&db, &minimal_test_run(1)).await;
            let second = insert_run(&db, &minimal_test_run(1)).await;

            let pending = db
                .get_testruns_after(TestKind::MixnodeStress, first)
                .await
                .unwrap();
            let ids: Vec<i64> = pending.iter().map(|run| run.id).collect();
            assert_eq!(ids, vec![second]);
        }
    }

    /// Every read of a kind's results is a hand-written query naming that kind's table, and the two
    /// mixnode tables have identical columns, so a query naming the wrong one of the pair would
    /// compile. Pin that each kind's readers see exactly that kind's runs.
    mod per_kind_tables {
        use super::*;

        #[tokio::test]
        async fn a_run_is_visible_only_to_its_own_kinds_readers() {
            for written in TestKind::iter() {
                let db = setup().await;
                seed_nodes(&db, &[described_node(1, "1.2.3.4", true, true)]).await;
                let id = insert_run_of(&db, written, &minimal_test_run(1)).await;

                for reader in TestKind::iter() {
                    let expected = usize::from(reader == written);
                    let context = format!("{written} run read as {reader}");

                    let by_id = db.get_testrun_by_id(reader, id).await.unwrap();
                    assert_eq!(usize::from(by_id.is_some()), expected, "{context}");

                    let latest = db.get_latest_testrun_for_node(reader, 1).await.unwrap();
                    assert_eq!(usize::from(latest.is_some()), expected, "{context}");

                    let (page, total) = db.get_testruns_paginated(reader, 10, 0).await.unwrap();
                    assert_eq!(
                        (page.len(), total),
                        (expected, expected as i64),
                        "{context}"
                    );

                    let (page, total) = db
                        .get_testruns_for_node_paginated(reader, 1, 10, 0)
                        .await
                        .unwrap();
                    assert_eq!(
                        (page.len(), total),
                        (expected, expected as i64),
                        "{context}"
                    );

                    let pending = db.get_testruns_after(reader, 0).await.unwrap();
                    assert_eq!(pending.len(), expected, "{context}");

                    let window = TestRunWindow {
                        start: datetime!(2025-06-01 00:00:00 UTC),
                        end: datetime!(2025-06-02 00:00:00 UTC),
                    };
                    let in_window = db.get_testruns_in_window(reader, window).await.unwrap();
                    assert_eq!(in_window.len(), expected, "{context}");
                }
            }
        }
    }

    mod get_testruns_in_window {
        use super::*;

        // an epoch's window ends where the next epoch's windows start from, so a run stored exactly
        // on a bound has to belong to the window starting there and not to the one ending there
        #[tokio::test]
        async fn includes_its_lower_bound_and_excludes_its_upper() {
            let db = setup().await;
            seed_node(&db, 1).await;
            let window = TestRunWindow {
                start: datetime!(2025-06-01 00:00:00 UTC),
                end: datetime!(2025-06-02 00:00:00 UTC),
            };

            let at_start = NewTestRun {
                test_timestamp: window.start,
                ..minimal_test_run(1)
            };
            let at_end = NewTestRun {
                test_timestamp: window.end,
                ..minimal_test_run(1)
            };
            let at_start_id = insert_run(&db, &at_start).await;
            insert_run(&db, &at_end).await;

            let runs = db
                .get_testruns_in_window(TestKind::MixnodeStress, window)
                .await
                .unwrap();
            let ids: Vec<i64> = runs.iter().map(|run| run.id).collect();
            assert_eq!(ids, vec![at_start_id]);
        }
    }

    mod mixnet_epoch_aggregate {
        use super::*;

        const MIXNET_EPOCH: i64 = 7;
        const EPOCH_START: OffsetDateTime = datetime!(2025-06-01 12:00:00 UTC);

        fn aggregate(node_id: i64, test_kind: TestKind, score: f64) -> MixnetEpochAggregate {
            MixnetEpochAggregate {
                mixnet_epoch: MIXNET_EPOCH,
                epoch_start: EPOCH_START,
                node_id,
                test_kind,
                score,
                samples: 12,
            }
        }

        // a second pass over the same window sees the results that have arrived since, so it
        // computes a different mean over more samples. what was served must not follow it
        #[tokio::test]
        async fn re_materialising_an_epoch_neither_duplicates_nor_alters_it() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let first = aggregate(1, TestKind::MixnodeStress, 0.9);
            db.batch_insert_mixnet_epoch_aggregates(&[first])
                .await
                .unwrap();

            let recomputed = MixnetEpochAggregate {
                score: 0.5,
                samples: 20,
                ..first
            };
            db.batch_insert_mixnet_epoch_aggregates(&[recomputed])
                .await
                .unwrap();

            assert_eq!(
                db.get_mixnet_epoch_aggregates(MIXNET_EPOCH).await.unwrap(),
                vec![first]
            );
        }

        // the per-node read backs an endpoint keyed by node, so a missing filter would serve one
        // operator another's numbers
        #[tokio::test]
        async fn a_node_reads_back_every_kind_that_measured_it_and_nothing_else() {
            let db = setup().await;
            seed_node(&db, 1).await;
            seed_node(&db, 2).await;

            let mixnode_liveness = aggregate(1, TestKind::MixnodeLiveness, 0.8);
            let gateway_liveness = aggregate(1, TestKind::GatewayLiveness, 0.7);
            let mixnode_stress = aggregate(1, TestKind::MixnodeStress, 0.9);
            let other_node = aggregate(2, TestKind::MixnodeStress, 0.1);
            let later_epoch = MixnetEpochAggregate {
                mixnet_epoch: MIXNET_EPOCH + 1,
                ..mixnode_stress
            };
            db.batch_insert_mixnet_epoch_aggregates(&[
                mixnode_liveness,
                gateway_liveness,
                mixnode_stress,
                other_node,
                later_epoch,
            ])
            .await
            .unwrap();

            assert_eq!(
                db.get_mixnet_epoch_aggregates_for_node(MIXNET_EPOCH, 1)
                    .await
                    .unwrap(),
                // ordered by the stored kind name
                vec![gateway_liveness, mixnode_liveness, mixnode_stress]
            );
        }

        #[tokio::test]
        async fn evicts_only_epochs_that_began_before_the_cutoff() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let old = aggregate(1, TestKind::MixnodeStress, 0.9);
            let at_cutoff = MixnetEpochAggregate {
                mixnet_epoch: MIXNET_EPOCH + 1,
                epoch_start: EPOCH_START + time::Duration::hours(1),
                ..old
            };
            db.batch_insert_mixnet_epoch_aggregates(&[old, at_cutoff])
                .await
                .unwrap();

            let evicted = db
                .evict_old_mixnet_epoch_aggregates(at_cutoff.epoch_start)
                .await
                .unwrap();

            assert_eq!(evicted, 1);
            assert!(
                db.get_mixnet_epoch_aggregates(MIXNET_EPOCH)
                    .await
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(
                db.get_mixnet_epoch_aggregates(MIXNET_EPOCH + 1)
                    .await
                    .unwrap(),
                vec![at_cutoff]
            );
        }
    }

    mod node_chain_capability {
        use super::*;

        fn capability(node_id: i64, next_refresh_due_at: OffsetDateTime) -> NodeChainCapability {
            NodeChainCapability {
                node_id,
                balance: "1000000unym".to_string(),
                is_feegrant_grantee: false,
                refreshed_at: FIXTURE_SEEN_AT,
                next_refresh_due_at,
            }
        }

        // the sweep queries exactly the described nodes with nothing current cached: an undescribed
        // node has no address to look up, and one cached until later is left alone
        #[tokio::test]
        async fn only_described_nodes_without_a_current_standing_await_refresh() {
            let db = setup().await;
            let now = datetime!(2025-06-01 12:00:00 UTC);
            seed_nodes(
                &db,
                &[mixnode(1), mixnode(2), mixnode(3), bond_only(mixnode(4))],
            )
            .await;
            db.batch_upsert_node_chain_capabilities(&[
                capability(2, now + time::Duration::hours(1)),
                capability(3, now),
            ])
            .await
            .unwrap();

            let awaiting: Vec<_> = db
                .get_nodes_awaiting_capability_refresh(now)
                .await
                .unwrap()
                .into_iter()
                .map(|node| node.node_id)
                .collect();

            // 1 was never queried and 3 fell due exactly now
            assert_eq!(awaiting, vec![1, 3]);
        }

        // a refresh replaces the cached standing rather than adding to it, and a candidate carries
        // whatever is cached, or nothing when nothing is
        #[tokio::test]
        async fn a_candidate_carries_its_latest_cached_standing() {
            let db = setup().await;
            seed_nodes(&db, &[mixnode(1), mixnode(2), bond_only(mixnode(3))]).await;

            let due = datetime!(2025-06-02 00:00:00 UTC);
            db.batch_upsert_node_chain_capabilities(&[capability(1, due)])
                .await
                .unwrap();
            db.batch_upsert_node_chain_capabilities(&[NodeChainCapability {
                balance: "5unym".to_string(),
                is_feegrant_grantee: true,
                ..capability(1, due)
            }])
            .await
            .unwrap();

            let candidates = db.get_config_score_candidates().await.unwrap();

            // the undescribed node is not a candidate at all
            let ids: Vec<_> = candidates.iter().map(|c| c.node_id).collect();
            assert_eq!(ids, vec![1, 2]);

            let cached = &candidates[0];
            assert_eq!(cached.reported_version, "1.1.0");
            assert!(cached.accepted_terms_and_conditions);
            assert_eq!(cached.balance.as_deref(), Some("5unym"));
            assert_eq!(cached.is_feegrant_grantee, Some(true));

            assert_eq!(candidates[1].balance, None);
            assert_eq!(candidates[1].is_feegrant_grantee, None);
        }
    }

    mod mixnet_epoch_config_score {
        use super::*;

        const MIXNET_EPOCH: i64 = 7;
        const EPOCH_START: OffsetDateTime = datetime!(2025-06-01 12:00:00 UTC);

        fn config_score(node_id: i64, score: f64) -> MixnetEpochConfigScore {
            MixnetEpochConfigScore {
                mixnet_epoch: MIXNET_EPOCH,
                epoch_start: EPOCH_START,
                node_id,
                score,
                versions_behind: Some(3),
                accepted_terms_and_conditions: true,
                runs_nym_node_binary: true,
                has_sufficient_tokens: true,
                is_feegrant_grantee: false,
            }
        }

        // a later pass recomputes from inputs that have changed since; what was served must stand
        #[tokio::test]
        async fn re_materialising_an_epoch_neither_duplicates_nor_alters_it() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let first = config_score(1, 0.9);
            db.batch_insert_mixnet_epoch_config_scores(&[first])
                .await
                .unwrap();

            let recomputed = MixnetEpochConfigScore {
                score: 0.5,
                versions_behind: Some(10),
                has_sufficient_tokens: false,
                ..first
            };
            db.batch_insert_mixnet_epoch_config_scores(&[recomputed])
                .await
                .unwrap();

            assert_eq!(
                db.get_mixnet_epoch_config_scores(MIXNET_EPOCH)
                    .await
                    .unwrap(),
                vec![first]
            );
        }

        // the decomposition is what attributes a low score, so it has to survive the round trip, and
        // the point read must not serve another node's or another epoch's row
        #[tokio::test]
        async fn a_node_reads_back_its_own_decomposition() {
            let db = setup().await;
            seed_node(&db, 1).await;
            seed_node(&db, 2).await;

            let scored = MixnetEpochConfigScore {
                versions_behind: None,
                accepted_terms_and_conditions: false,
                runs_nym_node_binary: true,
                has_sufficient_tokens: false,
                is_feegrant_grantee: true,
                ..config_score(1, 0.0)
            };
            let other_node = config_score(2, 0.8);
            let later_epoch = MixnetEpochConfigScore {
                mixnet_epoch: MIXNET_EPOCH + 1,
                ..config_score(1, 0.7)
            };
            db.batch_insert_mixnet_epoch_config_scores(&[scored, other_node, later_epoch])
                .await
                .unwrap();

            assert_eq!(
                db.get_mixnet_epoch_config_score_for_node(MIXNET_EPOCH, 1)
                    .await
                    .unwrap(),
                Some(scored)
            );
        }

        #[tokio::test]
        async fn evicts_only_epochs_that_began_before_the_cutoff() {
            let db = setup().await;
            seed_node(&db, 1).await;

            let old = config_score(1, 0.9);
            let at_cutoff = MixnetEpochConfigScore {
                mixnet_epoch: MIXNET_EPOCH + 1,
                epoch_start: EPOCH_START + time::Duration::hours(1),
                ..old
            };
            db.batch_insert_mixnet_epoch_config_scores(&[old, at_cutoff])
                .await
                .unwrap();

            let evicted = db
                .evict_old_mixnet_epoch_config_scores(at_cutoff.epoch_start)
                .await
                .unwrap();

            assert_eq!(evicted, 1);
            assert!(
                db.get_mixnet_epoch_config_scores(MIXNET_EPOCH)
                    .await
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(
                db.get_mixnet_epoch_config_scores(MIXNET_EPOCH + 1)
                    .await
                    .unwrap(),
                vec![at_cutoff]
            );
        }
    }
}
