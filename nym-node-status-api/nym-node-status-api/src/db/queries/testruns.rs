use crate::db::models::{TestRunDto, TestRunStatus};
use crate::db::DbConnection;
use crate::db::DbPool;
use crate::http::models::TestrunAssignment;
use crate::utils::now_utc;
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
    let now = now_utc().unix_timestamp();
    // find & mark as "In progress" in the same transaction to avoid race conditions
    let returning = sqlx::query!(
        r#"UPDATE testruns
            SET
                status = $1,
                last_assigned_utc = $2
            WHERE id =
        (
            SELECT id
            FROM testruns
            WHERE status = $3
            ORDER BY created_utc asc
            LIMIT 1
        )
        RETURNING
            id as "id!",
            gateway_id
            "#,
        TestRunStatus::InProgress as i32,
        now,
        TestRunStatus::Queued as i32,
    )
    .fetch_optional(conn.as_mut())
    .await?;

    if let Some(testrun) = returning {
        let gw_identity = sqlx::query!(
            r#"
                SELECT
                    id,
                    gateway_identity_key
                FROM gateways
                WHERE id = $1
                LIMIT 1"#,
            testrun.gateway_id
        )
        .fetch_one(conn.as_mut())
        .await?;

        Ok(Some(TestrunAssignment {
            testrun_id: testrun.id,
            gateway_identity_key: gw_identity.gateway_identity_key,
            assigned_at_utc: now,
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
    sqlx::query!(
        "UPDATE gateways SET last_probe_result = $1 WHERE id = $2",
        result,
        gateway_pk,
    )
    .execute(conn.as_mut())
    .await
    .map(drop)
    .map_err(From::from)
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
            created_utc,
            last_assigned_utc,
            ip_address,
            log
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        testrun_id,
        gateway_id,
        TestRunStatus::InProgress as i32,
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
