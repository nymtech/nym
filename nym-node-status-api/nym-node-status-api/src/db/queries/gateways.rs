use crate::{
    db::{
        models::{GatewayDto, GatewayInsertRecord},
        DbPool,
    },
    http::models::Gateway,
};
use futures_util::TryStreamExt;
use sqlx::{pool::PoolConnection, Sqlite};
use tracing::error;

pub(crate) async fn select_gateway_identity(
    conn: &mut PoolConnection<Sqlite>,
    gateway_pk: i64,
) -> anyhow::Result<String> {
    let record = sqlx::query!(
        r#"SELECT
            gateway_identity_key
        FROM
            gateways
        WHERE
            id = ?"#,
        gateway_pk
    )
    .fetch_one(conn.as_mut())
    .await?;

    Ok(record.gateway_identity_key)
}

pub(crate) async fn insert_gateways(
    pool: &DbPool,
    gateways: Vec<GatewayInsertRecord>,
) -> anyhow::Result<()> {
    let mut db = pool.acquire().await?;
    for record in gateways {
        sqlx::query!(
            "INSERT INTO gateways
                (gateway_identity_key, bonded,
                    self_described, explorer_pretty_bond,
                    last_updated_utc, performance)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT(gateway_identity_key) DO UPDATE SET
                bonded=excluded.bonded,
                self_described=excluded.self_described,
                explorer_pretty_bond=excluded.explorer_pretty_bond,
                last_updated_utc=excluded.last_updated_utc,
                performance = excluded.performance;",
            record.identity_key,
            record.bonded,
            record.self_described,
            record.explorer_pretty_bond,
            record.last_updated_utc,
            record.performance
        )
        .execute(&mut *db)
        .await?;
    }

    Ok(())
}

pub(crate) async fn get_all_gateways(pool: &DbPool) -> anyhow::Result<Vec<Gateway>> {
    let mut conn = pool.acquire().await?;
    let items = sqlx::query_as!(
        GatewayDto,
        r#"SELECT
            gw.gateway_identity_key as "gateway_identity_key!",
            gw.bonded as "bonded: bool",
            gw.performance as "performance!",
            gw.self_described as "self_described?",
            gw.explorer_pretty_bond as "explorer_pretty_bond?",
            gw.last_probe_result as "last_probe_result?",
            gw.last_probe_log as "last_probe_log?",
            gw.last_testrun_utc as "last_testrun_utc?",
            gw.last_updated_utc as "last_updated_utc!",
            COALESCE(gd.moniker, "NA") as "moniker!",
            COALESCE(gd.website, "NA") as "website!",
            COALESCE(gd.security_contact, "NA") as "security_contact!",
            COALESCE(gd.details, "NA") as "details!"
         FROM gateways gw
         LEFT JOIN gateway_description gd
         ON gw.gateway_identity_key = gd.gateway_identity_key
         ORDER BY gw.gateway_identity_key"#,
    )
    .fetch(&mut *conn)
    .try_collect::<Vec<_>>()
    .await?;

    let items: Vec<Gateway> = items
        .into_iter()
        .map(|item| item.try_into())
        .collect::<anyhow::Result<Vec<_>>>()
        .map_err(|e| {
            error!("Conversion from DTO failed: {e}. Invalidly stored data?");
            e
        })?;
    tracing::trace!("Fetched {} gateways from DB", items.len());
    Ok(items)
}
