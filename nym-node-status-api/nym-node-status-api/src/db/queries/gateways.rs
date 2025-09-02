use std::collections::HashSet;

use crate::{
    db::{
        models::{GatewayDto, GatewayInsertRecord},
        DbConnection, DbPool,
    },
    http::models::Gateway,
    node_scraper::helpers::NodeDescriptionResponse,
};
use futures_util::TryStreamExt;
use sqlx::{pool::PoolConnection, Postgres};
use tracing::error;

pub(crate) async fn select_gateway_identity(
    conn: &mut PoolConnection<Postgres>,
    gateway_pk: i32,
) -> anyhow::Result<String> {
    let record = sqlx::query!(
        r#"SELECT
            gateway_identity_key
        FROM
            gateways
        WHERE
            id = $1"#,
        gateway_pk
    )
    .fetch_one(conn.as_mut())
    .await?;

    Ok(record.gateway_identity_key)
}

pub(crate) async fn update_bonded_gateways(
    pool: &DbPool,
    gateways: Vec<GatewayInsertRecord>,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        r#"UPDATE
            gateways
        SET
            bonded = false
        "#,
    )
    .execute(&mut *tx)
    .await?;

    for record in gateways {
        sqlx::query!(
            "INSERT INTO gateways
                (gateway_identity_key, bonded,
                    self_described, explorer_pretty_bond,
                    last_updated_utc, performance)
                VALUES ($1, $2, $3, $4, $5, $6)
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
            record.performance as i32
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

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
            COALESCE(gd.moniker, 'NA') as "moniker!",
            COALESCE(gd.website, 'NA') as "website!",
            COALESCE(gd.security_contact, 'NA') as "security_contact!",
            COALESCE(gd.details, 'NA') as "details!"
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
        .inspect_err(|e| error!("Conversion from DTO failed: {e}. Invalidly stored data?"))?;
    tracing::trace!("Fetched {} gateways from DB", items.len());
    Ok(items)
}

pub(crate) async fn get_bonded_gateway_id_keys(pool: &DbPool) -> anyhow::Result<HashSet<String>> {
    let mut conn = pool.acquire().await?;
    let items = sqlx::query!(
        r#"
            SELECT gateway_identity_key
            FROM gateways
            WHERE bonded = true
        "#
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|record| record.gateway_identity_key)
    .collect::<HashSet<_>>();

    Ok(items)
}

pub(crate) async fn insert_gateway_description(
    conn: &mut PoolConnection<Postgres>,
    identity_key: &str,
    description: &NodeDescriptionResponse,
    timestamp: i64,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO gateway_description (
            gateway_identity_key,
            moniker,
            website,
            security_contact,
            details,
            last_updated_utc
        ) VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (gateway_identity_key) DO UPDATE SET
            moniker = excluded.moniker,
            website = excluded.website,
            security_contact = excluded.security_contact,
            details = excluded.details,
            last_updated_utc = excluded.last_updated_utc
        "#,
        identity_key,
        description.moniker,
        description.website,
        description.security_contact,
        description.details,
        timestamp,
    )
    .execute(conn.as_mut())
    .await
    .map(drop)
    .map_err(From::from)
}

pub(crate) async fn get_or_create_gateway(
    conn: &mut DbConnection,
    gateway_identity_key: &str,
) -> anyhow::Result<i32> {
    // Try to find existing gateway
    let existing = sqlx::query_scalar!(
        "SELECT id FROM gateways WHERE gateway_identity_key = $1",
        gateway_identity_key.to_string()
    )
    .fetch_optional(conn.as_mut())
    .await?;

    if let Some(row) = existing {
        return Ok(row);
    }

    // Create new gateway
    tracing::info!("Creating new gateway record for {}", gateway_identity_key);
    let now = crate::utils::now_utc().unix_timestamp();

    let result: i32 = sqlx::query_scalar!(
        r#"INSERT INTO gateways (
            gateway_identity_key,
            bonded,
            performance,
            self_described,
            last_updated_utc
        ) VALUES ($1, $2, $3, $4, $5)
        RETURNING id"#,
        gateway_identity_key.to_string(),
        true, // Assume bonded since being tested
        0,    // Initial performance
        "null",
        now
    )
    .fetch_one(conn.as_mut())
    .await?;

    Ok(result)
}
