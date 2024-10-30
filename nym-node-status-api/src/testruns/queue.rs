use crate::db::models::{GatewayInfoDto, TestRunDto, TestRunStatus};
use crate::testruns::models::TestRun;
use anyhow::anyhow;
use chrono::DateTime;
use futures_util::TryStreamExt;
use sqlx::pool::PoolConnection;
use sqlx::Sqlite;
use std::time::SystemTime;

pub(crate) async fn try_queue_testrun(
    conn: &mut PoolConnection<Sqlite>,
    identity_key: String,
    ip_address: String,
) -> anyhow::Result<TestRun> {
    let timestamp = now_utc().timestamp();
    let timestamp_pretty = now_utc_as_rfc3339();

    let items = sqlx::query_as!(
        GatewayInfoDto,
        r#"SELECT
            id as "id!",
            gateway_identity_key as "gateway_identity_key!",
            self_described as "self_described?",
            explorer_pretty_bond as "explorer_pretty_bond?"
         FROM gateways
         WHERE gateway_identity_key = ?
         ORDER BY gateway_identity_key
         LIMIT 1"#,
        identity_key,
    )
    // TODO dz shoudl call .fetch_one
    // TODO dz replace this in other queries as well
    .fetch(conn.as_mut())
    .try_collect::<Vec<_>>()
    .await?;

    let gateway = items
        .iter()
        .find(|g| g.gateway_identity_key == identity_key);

    // TODO dz if let Some() = gateway.first() ...
    if gateway.is_none() {
        return Err(anyhow!("Unknown gateway {identity_key}"));
    }
    let gateway_id = gateway.unwrap().id;

    //
    // check if there is already a test run for this gateway
    //
    let items = sqlx::query_as!(
        TestRunDto,
        r#"SELECT
            id as "id!",
            gateway_id as "gateway_id!",
            status as "status!",
            timestamp_utc as "timestamp_utc!",
            ip_address as "ip_address!",
            log as "log!"
         FROM testruns
         WHERE gateway_id = ? AND status != 2
         ORDER BY id DESC
         LIMIT 1"#,
        gateway_id,
    )
    .fetch(conn.as_mut())
    .try_collect::<Vec<_>>()
    .await?;

    if !items.is_empty() {
        let testrun = items.first().unwrap();
        return Ok(TestRun {
            id: testrun.id as u32,
            identity_key,
            status: format!(
                "{}",
                TestRunStatus::from_repr(testrun.status as u8).unwrap()
            ),
            log: testrun.log.clone(),
        });
    }

    //
    // save test run
    //
    let status = TestRunStatus::Queued as u32;
    let log = format!(
        "Test for {identity_key} requested at {} UTC\n\n",
        timestamp_pretty
    );

    let id = sqlx::query!(
        "INSERT INTO testruns (gateway_id, status, ip_address, timestamp_utc, log) VALUES (?, ?, ?, ?, ?)",
        gateway_id,
        status,
        ip_address,
        timestamp,
        log,
    )
        .execute(conn.as_mut())
        .await?
        .last_insert_rowid();

    Ok(TestRun {
        id: id as u32,
        identity_key,
        status: format!("{}", TestRunStatus::Queued),
        log,
    })
}

// TODO dz do we need these?
pub fn now_utc() -> DateTime<chrono::Utc> {
    SystemTime::now().into()
}

pub fn now_utc_as_rfc3339() -> String {
    now_utc().to_rfc3339()
}
