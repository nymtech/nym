use crate::{
    db::models::{TestRunDto, TestRunStatus},
    testruns::now_utc,
};
use anyhow::Context;
use futures_util::TryStreamExt;
use nym_bin_common::models::ns_api::TestrunAssignment;
use serde::Deserialize;
use sqlx::{pool::PoolConnection, Sqlite};

pub(crate) async fn get_testruns(conn: PoolConnection<Sqlite>) -> anyhow::Result<Vec<TestRunDto>> {
    // TODO dz accept mut reference, repeat in all similar functions
    let mut conn = conn;
    let testruns = sqlx::query_as!(
        TestRunDto,
        r#"SELECT
            id as "id!",
            gateway_id as "gateway_id!",
            status as "status!",
            timestamp_utc as "timestamp_utc!",
            ip_address as "ip_address!",
            log as "log!"
         FROM testruns
         WHERE status = 0
         ORDER BY timestamp_utc"#
    )
    .fetch(&mut *conn)
    .try_collect::<Vec<_>>()
    .await?;

    Ok(testruns)
}

pub(crate) async fn get_testrun_by_id(
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
         WHERE id = ?
         ORDER BY timestamp_utc"#,
        testrun_id
    )
    .fetch_one(&mut *conn)
    .await
    .context(format!("Couldn't retrieve testrun {testrun_id}"))
}

pub(crate) async fn get_oldest_testrun_and_make_it_pending(
    // TODO dz accept mut reference, repeat in all similar functions
    conn: PoolConnection<Sqlite>,
) -> anyhow::Result<Option<TestrunAssignment>> {
    let mut conn = conn;
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
        TestRunStatus::Pending as i64,
    )
    .fetch_optional(&mut *conn)
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
    .execute(&mut *conn)
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
    .execute(&mut *conn)
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
    .execute(&mut *conn)
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
    .execute(&mut *conn)
    .await
    .map(drop)
    .map_err(From::from)
}
