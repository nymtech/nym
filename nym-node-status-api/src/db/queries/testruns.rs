use crate::db::DbPool;
use crate::http::models::TestrunAssignment;
use crate::{
    db::models::{TestRunDto, TestRunStatus},
    testruns::now_utc,
};
use chrono::Duration;
use nym_crypto::asymmetric::ed25519::PublicKey;
use sqlx::{pool::PoolConnection, Sqlite};

pub(crate) async fn testrun_in_progress_assigned_to_agent(
    conn: &mut PoolConnection<Sqlite>,
    agent_key: &PublicKey,
) -> sqlx::Result<TestRunDto> {
    let agent_key = agent_key.to_base58_string();
    sqlx::query_as!(
        TestRunDto,
        r#"SELECT
            id as "id!",
            gateway_id as "gateway_id!",
            status as "status!",
            created_utc as "created_utc!",
            ip_address as "ip_address!",
            log as "log!",
            assigned_agent,
            last_assigned_utc
         FROM testruns
         WHERE
            assigned_agent = ?
         AND
            status = ?
         ORDER BY created_utc"#,
        agent_key,
        TestRunStatus::InProgress as i64,
    )
    .fetch_one(conn.as_mut())
    .await
}

pub(crate) async fn update_testruns_assigned_before(
    db: &DbPool,
    max_age: Duration,
) -> anyhow::Result<u64> {
    let mut conn = db.acquire().await?;
    let previous_run = now_utc() - max_age;
    let cutoff_timestamp = previous_run.timestamp();

    let res = sqlx::query!(
        r#"UPDATE
            testruns
        SET
            status = ?,
            assigned_agent = NULL
        WHERE
            status = ?
        AND
            last_assigned_utc < ?
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
    conn: &mut PoolConnection<Sqlite>,
    agent_key: PublicKey,
) -> anyhow::Result<Option<TestrunAssignment>> {
    let agent_key = agent_key.to_base58_string();
    let now = now_utc().timestamp();
    // find & mark as "In progress" in the same transaction to avoid race conditions
    let returning = sqlx::query!(
        r#"UPDATE testruns
            SET
                status = ?,
                assigned_agent = ?,
                last_assigned_utc = ?
            WHERE rowid =
        (
            SELECT rowid
            FROM testruns
            WHERE status = ?
            ORDER BY created_utc asc
            LIMIT 1
        )
        RETURNING
            id as "id!",
            gateway_id
            "#,
        TestRunStatus::InProgress as i64,
        agent_key,
        now,
        TestRunStatus::Queued as i64,
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
                WHERE id = ?
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
