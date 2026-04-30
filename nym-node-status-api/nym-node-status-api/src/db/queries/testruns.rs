use crate::db::DbConnection;
use crate::db::DbPool;
use crate::db::models::{TestRunDto, TestRunKind, TestRunStatus};
use crate::http::models::TestrunAssignment;
use crate::utils::now_utc;
use sqlx::Row;
use sqlx::types::Json;
use time::Duration;

pub(crate) async fn count_testruns_in_progress(
    conn: &mut DbConnection,
) -> anyhow::Result<Option<i64>> {
    sqlx::query_scalar!(
        r#"SELECT
            COUNT(id) as "count: i64"
         FROM testruns
         WHERE
            status = $1
         "#,
        TestRunStatus::InProgress as i64,
    )
    .fetch_one(conn.as_mut())
    .await
    .map_err(anyhow::Error::from)
}

pub(crate) async fn get_in_progress_testrun_by_id(
    conn: &mut DbConnection,
    testrun_id: i32,
) -> anyhow::Result<TestRunDto> {
    sqlx::query_as!(
        TestRunDto,
        r#"SELECT
            id as "id!",
            gateway_id as "gateway_id!",
            status as "status!",
            kind as "kind!",
            created_utc as "created_utc!",
            ip_address as "ip_address!",
            log as "log!",
            last_assigned_utc
         FROM testruns
         WHERE
            id = $1
         AND
            status = $2
         ORDER BY created_utc
         LIMIT 1"#,
        testrun_id,
        TestRunStatus::InProgress as i64,
    )
    .fetch_one(conn.as_mut())
    .await
    .map_err(|e| anyhow::anyhow!("Couldn't retrieve testrun {testrun_id}: {e}"))
}

pub(crate) async fn update_testruns_assigned_before(
    db: &DbPool,
    max_age: Duration,
) -> anyhow::Result<u64> {
    let mut conn = db.acquire().await?;
    let previous_run = now_utc() - max_age;
    let cutoff_timestamp = previous_run.unix_timestamp();

    let res = sqlx::query!(
        r#"UPDATE
            testruns
        SET
            status = $1
        WHERE
            status = $2
        AND
            last_assigned_utc < $3
            "#,
        TestRunStatus::Queued as i64,
        TestRunStatus::InProgress as i64,
        cutoff_timestamp
    )
    .execute(conn.as_mut())
    .await?;

    let stale_testruns = res.rows_affected();
    if stale_testruns > 0 {
        tracing::info!(
            "Refreshed {} stale testruns, assigned before {} but not yet finished",
            stale_testruns,
            previous_run
        );
    }

    Ok(stale_testruns)
}

pub(crate) async fn assign_oldest_testrun(
    conn: &mut DbConnection,
) -> anyhow::Result<Option<TestrunAssignment>> {
    assign_oldest_testrun_by_kind(conn, TestRunKind::Probe).await
}

pub(crate) async fn assign_oldest_ports_check_testrun(
    conn: &mut DbConnection,
) -> anyhow::Result<Option<TestrunAssignment>> {
    assign_oldest_testrun_by_kind(conn, TestRunKind::PortsCheck).await
}

async fn assign_oldest_testrun_by_kind(
    conn: &mut DbConnection,
    kind: TestRunKind,
) -> anyhow::Result<Option<TestrunAssignment>> {
    let now = now_utc().unix_timestamp();
    // find & mark as "In progress" in the same transaction to avoid race conditions
    // lock the row to avoid two threads reading the same value
    let returning = sqlx::query!(
        r#"
        WITH oldest_queued AS (
            SELECT id
            FROM testruns
            WHERE status = $1 AND kind = $4
            ORDER BY created_utc asc
            LIMIT 1
            FOR UPDATE SKIP LOCKED
        )
        UPDATE testruns
            SET
                status = $3,
                last_assigned_utc = $2
            FROM oldest_queued
            WHERE testruns.id = oldest_queued.id
        RETURNING
            testruns.id,
            testruns.gateway_id
            "#,
        TestRunStatus::Queued as i32,
        now,
        TestRunStatus::InProgress as i32,
        kind as i16,
    )
    .fetch_optional(conn.as_mut())
    .await?;

    if let Some(testrun) = returning {
        let row = sqlx::query(
            r#"SELECT gateway_identity_key, last_ports_check_utc FROM gateways WHERE id = $1 LIMIT 1"#,
        )
        .bind(testrun.gateway_id)
        .fetch_one(conn.as_mut())
        .await?;

        let gateway_identity_key: String = row.try_get("gateway_identity_key")?;
        let last_ports_check_utc: Option<i64> = row.try_get("last_ports_check_utc")?;

        Ok(Some(TestrunAssignment {
            testrun_id: testrun.id,
            gateway_identity_key,
            assigned_at_utc: now,
            last_ports_check_utc,
        }))
    } else {
        Ok(None)
    }
}

pub(crate) async fn update_testrun_status(
    conn: &mut DbConnection,
    testrun_id: i32,
    status: TestRunStatus,
) -> anyhow::Result<()> {
    let status = status as i32;
    sqlx::query!(
        "UPDATE testruns SET status = $1 WHERE id = $2",
        status,
        testrun_id,
    )
    .execute(conn.as_mut())
    .await?;

    Ok(())
}

pub(crate) async fn update_gateway_last_probe_log(
    conn: &mut DbConnection,
    gateway_pk: i32,
    log: &str,
) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE gateways SET last_probe_log = $1 WHERE id = $2",
        log,
        gateway_pk,
    )
    .execute(conn.as_mut())
    .await
    .map(drop)
    .map_err(From::from)
}

pub(crate) async fn update_gateway_last_probe_result(
    conn: &mut DbConnection,
    gateway_pk: i32,
    result: &str,
) -> anyhow::Result<()> {
    use crate::db::models::detach_ports_check_from_probe_json;

    let value: serde_json::Value = serde_json::from_str(result)
        .map_err(|e| anyhow::anyhow!("Invalid probe result JSON: {e}"))?;
    let (stripped_json, ports_check) = detach_ports_check_from_probe_json(value);
    let stripped = serde_json::to_string(&stripped_json)?;
    let now_ts = crate::utils::now_utc().unix_timestamp();
    let ports_check_ts = ports_check.as_ref().map(|_| now_ts);

    sqlx::query(
        r#"UPDATE gateways SET
            last_probe_result = $1,
            ports_check = COALESCE($2, ports_check),
            last_ports_check_utc = CASE WHEN $2 IS NOT NULL THEN $3 ELSE last_ports_check_utc END
        WHERE id = $4"#,
    )
    .bind(&stripped)
    .bind(ports_check.map(Json))
    .bind(ports_check_ts)
    .bind(gateway_pk)
    .execute(conn.as_mut())
    .await
    .map(drop)
    .map_err(From::from)
}

// NOTE: port-check submissions must not re-embed `ports_check` into `last_probe_result`.

pub(crate) async fn update_gateway_ports_check_only(
    conn: &mut DbConnection,
    gateway_pk: i32,
    port_check_result: &nym_gateway_probe::PortCheckResult,
) -> anyhow::Result<()> {
    use crate::db::models::ports_check_summary_json_from_result;

    let now_ts = now_utc().unix_timestamp();
    let value = ports_check_summary_json_from_result(port_check_result);

    sqlx::query(
        r#"UPDATE gateways SET
            ports_check = $1,
            last_ports_check_utc = $2
        WHERE id = $3"#,
    )
    .bind(Json(value))
    .bind(now_ts)
    .bind(gateway_pk)
    .execute(conn.as_mut())
    .await
    .map(drop)
    .map_err(From::from)
}

pub(crate) async fn insert_external_ports_check_testrun(
    conn: &mut DbConnection,
    testrun_id: i32,
    gateway_id: i32,
    assigned_at_utc: i64,
) -> anyhow::Result<()> {
    let now = now_utc().unix_timestamp();

    sqlx::query!(
        r#"INSERT INTO testruns (
            id,
            gateway_id,
            status,
            kind,
            created_utc,
            last_assigned_utc,
            ip_address,
            log
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        testrun_id,
        gateway_id,
        TestRunStatus::InProgress as i32,
        TestRunKind::PortsCheck as i16,
        now,
        assigned_at_utc,
        "external",
        ""
    )
    .execute(conn.as_mut())
    .await?;

    tracing::debug!(
        "Created external ports-check testrun {} for gateway {}",
        testrun_id,
        gateway_id
    );
    Ok(())
}

pub(crate) async fn enqueue_due_ports_check_testruns(db: &DbPool) -> anyhow::Result<u64> {
    let mut conn = db.acquire().await?;
    let now = now_utc().unix_timestamp();
    // 14 days soft TTL for dedicated ports-check queueing.
    let cutoff = now - time::Duration::days(14).whole_seconds();

    let res = sqlx::query!(
        r#"
        INSERT INTO testruns (gateway_id, status, kind, created_utc, last_assigned_utc, ip_address, log)
        SELECT
            gw.id,
            $1,
            $2,
            $3,
            NULL,
            'ports_check_scheduler',
            ''
        FROM gateways gw
        WHERE gw.bonded = true
          AND (gw.last_ports_check_utc IS NULL OR gw.last_ports_check_utc < $4)
          AND NOT EXISTS (
              SELECT 1
              FROM testruns t
              WHERE t.gateway_id = gw.id
                AND t.kind = $2
                AND t.status IN ($1, $5)
          )
        "#,
        TestRunStatus::Queued as i32,
        TestRunKind::PortsCheck as i16,
        now,
        cutoff,
        TestRunStatus::InProgress as i32,
    )
    .execute(conn.as_mut())
    .await?;

    Ok(res.rows_affected())
}

pub(crate) async fn update_gateway_score(
    conn: &mut DbConnection,
    gateway_pk: i32,
) -> anyhow::Result<()> {
    let now = now_utc().unix_timestamp();
    sqlx::query!(
        "UPDATE gateways SET last_testrun_utc = $1, last_updated_utc = $2 WHERE id = $3",
        now,
        now,
        gateway_pk,
    )
    .execute(conn.as_mut())
    .await
    .map(drop)
    .map_err(From::from)
}

pub(crate) async fn get_testrun_by_id(
    conn: &mut DbConnection,
    testrun_id: i32,
) -> anyhow::Result<TestRunDto> {
    sqlx::query_as!(
        TestRunDto,
        r#"SELECT
            id,
            gateway_id,
            status,
            kind,
            created_utc,
            ip_address,
            log,
            last_assigned_utc
         FROM testruns
         WHERE id = $1"#,
        testrun_id
    )
    .fetch_one(conn.as_mut())
    .await
    .map_err(|e| anyhow::anyhow!("Testrun {} not found: {}", testrun_id, e))
}

pub(crate) async fn insert_external_testrun(
    conn: &mut DbConnection,
    testrun_id: i32,
    gateway_id: i32,
    assigned_at_utc: i64,
) -> anyhow::Result<()> {
    let now = crate::utils::now_utc().unix_timestamp();

    sqlx::query!(
        r#"INSERT INTO testruns (
            id,
            gateway_id,
            status,
            kind,
            created_utc,
            last_assigned_utc,
            ip_address,
            log
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        testrun_id,
        gateway_id,
        TestRunStatus::InProgress as i32,
        TestRunKind::Probe as i16,
        now,
        assigned_at_utc,
        "external", // Marker for external origin
        ""
    ) // Empty initial log
    .execute(conn.as_mut())
    .await?;

    tracing::debug!(
        "Created external testrun {} for gateway {}",
        testrun_id,
        gateway_id
    );
    Ok(())
}

pub(crate) async fn update_testrun_status_by_gateway(
    conn: &mut DbConnection,
    gateway_id: i32,
    status: TestRunStatus,
) -> anyhow::Result<()> {
    let status = status as i32;
    sqlx::query!(
        "UPDATE testruns SET status = $1 WHERE gateway_id = $2 AND status = $3",
        status,
        gateway_id,
        TestRunStatus::InProgress as i32
    )
    .execute(conn.as_mut())
    .await?;

    Ok(())
}
