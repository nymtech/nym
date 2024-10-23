use crate::db::{
    models::{TestRunDto, TestRunStatus},
    DbPool,
};
use chrono::{DateTime, Utc};
use futures_util::TryStreamExt;
use sqlx::{pool::PoolConnection, Sqlite};
use std::collections::HashMap;

pub(crate) async fn get_testruns(conn: PoolConnection<Sqlite>) -> anyhow::Result<Vec<TestRunDto>> {
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
         ORDER BY id"#
    )
    .fetch(&mut *conn)
    .try_collect::<Vec<_>>()
    .await?;

    Ok(testruns)
}

pub(crate) async fn update_status(
    conn: PoolConnection<Sqlite>,
    task_id: u32,
    status: TestRunStatus,
) -> anyhow::Result<()> {
    let mut conn = conn;
    let status = status as u32;
    sqlx::query!(
        "UPDATE testruns SET status = ? WHERE id = ?",
        status,
        task_id
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}
