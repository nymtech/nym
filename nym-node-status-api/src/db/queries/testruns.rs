use crate::db::DbPool;
use crate::http::models::TestrunAssignment;
use crate::{
    db::models::{TestRunDto, TestRunStatus},
    testruns::now_utc,
};
use anyhow::Context;
use chrono::Duration;
use sqlx::{pool::PoolConnection, Sqlite};

pub(crate) async fn get_in_progress_testrun_by_id(
    conn: &mut PoolConnection<Sqlite>,
    testrun_id: i64,
) -> anyhow::Result<TestRunDto> {
    sqlx::query_as!(
        TestRunDto,
        r#"SELECT
            id as "id!",
            gateway_id as "gateway_id!",
            status as "status!",
            timestamp_utc as "timestamp_utc!",
            ip_address as "ip_address!",
            log as "log!"
         FROM testruns
         WHERE
            id = ?
         AND
            status = ?
         ORDER BY timestamp_utc"#,
        testrun_id,
        TestRunStatus::InProgress as i64,
    )
    .fetch_one(conn.as_mut())
    .await
    .context(format!("Couldn't retrieve testrun {testrun_id}"))
}

pub(crate) async fn update_testruns_older_than(db: &DbPool, age: Duration) -> anyhow::Result<u64> {
    let mut conn = db.acquire().await?;
    let previous_run = now_utc() - age;
    let cutoff_timestamp = previous_run.timestamp();

    let res = sqlx::query!(
        r#"UPDATE
            testruns
        SET
            status = ?
        WHERE
            status = ?
        AND
            timestamp_utc < ?
            "#,
        TestRunStatus::Queued as i64,
        TestRunStatus::InProgress as i64,
        cutoff_timestamp
    )
    .execute(conn.as_mut())
    .await?;

    let stale_testruns = res.rows_affected();
    if stale_testruns > 0 {
        tracing::debug!(
            "Refreshed {} stale testruns, scheduled before {} but not yet finished",
            stale_testruns,
            previous_run
        );
    }

    Ok(stale_testruns)
}

pub(crate) async fn get_oldest_testrun_and_make_it_pending(
    conn: &mut PoolConnection<Sqlite>,
) -> anyhow::Result<Option<TestrunAssignment>> {
    let assignment = sqlx::query_as!(
        TestrunAssignment,
        r#"UPDATE testruns
            SET status = ?
            WHERE rowid =
        (
            SELECT rowid
            FROM testruns
            WHERE status = ?
            ORDER BY timestamp_utc asc
            LIMIT 1
        )
        RETURNING
            id as "testrun_id!",
            gateway_id as "gateway_pk_id!"
            "#,
        TestRunStatus::InProgress as i64,
        TestRunStatus::Queued as i64,
    )
    .fetch_optional(conn.as_mut())
    .await?;

    Ok(assignment)
}

pub(crate) async fn update_testrun_status(
    conn: &mut PoolConnection<Sqlite>,
    testrun_id: i64,
    status: TestRunStatus,
) -> anyhow::Result<()> {
    let status = status as u32;
    sqlx::query!(
        "UPDATE testruns SET status = ? WHERE id = ?",
        status,
        testrun_id
    )
    .execute(conn.as_mut())
    .await?;

    Ok(())
}

pub(crate) async fn update_gateway_last_probe_log(
    conn: &mut PoolConnection<Sqlite>,
    gateway_pk: i64,
    log: &str,
) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE gateways SET last_probe_log = ? WHERE id = ?",
        log,
        gateway_pk
    )
    .execute(conn.as_mut())
    .await
    .map(drop)
    .map_err(From::from)
}

pub(crate) async fn update_gateway_last_probe_result(
    conn: &mut PoolConnection<Sqlite>,
    gateway_pk: i64,
    result: &str,
) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE gateways SET last_probe_result = ? WHERE id = ?",
        result,
        gateway_pk
    )
    .execute(conn.as_mut())
    .await
    .map(drop)
    .map_err(From::from)
}

pub(crate) async fn update_gateway_score(
    conn: &mut PoolConnection<Sqlite>,
    gateway_pk: i64,
) -> anyhow::Result<()> {
    let now = now_utc().timestamp();
    sqlx::query!(
        "UPDATE gateways SET last_testrun_utc = ?, last_updated_utc = ? WHERE id = ?",
        now,
        now,
        gateway_pk
    )
    .execute(conn.as_mut())
    .await
    .map(drop)
    .map_err(From::from)
}
