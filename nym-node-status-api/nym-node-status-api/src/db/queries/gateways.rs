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
use sqlx::Row;
use tracing::error;

pub(crate) async fn select_gateway_identity(
    conn: &mut DbConnection,
    gateway_pk: i64,
) -> anyhow::Result<String> {
    let record = crate::db::query(
        r#"SELECT
            gateway_identity_key
        FROM
            gateways
        WHERE
            id = ?"#,
    )
    .bind(gateway_pk)
    .fetch_one(conn.as_mut())
    .await?;

    Ok(record.try_get("gateway_identity_key")?)
}

pub(crate) async fn update_bonded_gateways(
    pool: &DbPool,
    gateways: Vec<GatewayInsertRecord>,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;

    crate::db::query(
        r#"UPDATE
            gateways
        SET
            bonded = false
        "#,
    )
    .execute(&mut *tx)
    .await?;

    for record in gateways {
        crate::db::query(
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
        )
        .bind(record.identity_key)
        .bind(record.bonded)
        .bind(record.self_described)
        .bind(record.explorer_pretty_bond)
        .bind(record.last_updated_utc)
        .bind(record.performance as i32)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(())
}

pub(crate) async fn get_all_gateways(pool: &DbPool) -> anyhow::Result<Vec<Gateway>> {
    let mut conn = pool.acquire().await?;
    let items = crate::db::query_as::<GatewayDto>(
        r#"SELECT
            gw.gateway_identity_key,
            gw.bonded,
            gw.performance,
            gw.self_described,
            gw.explorer_pretty_bond,
            gw.last_probe_result,
            gw.last_probe_log,
            gw.last_testrun_utc,
            gw.last_updated_utc,
            COALESCE(gd.moniker, 'NA') as moniker,
            COALESCE(gd.website, 'NA') as website,
            COALESCE(gd.security_contact, 'NA') as security_contact,
            COALESCE(gd.details, 'NA') as details
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
    let items = crate::db::query(
        r#"
            SELECT gateway_identity_key
            FROM gateways
            WHERE bonded = true
        "#,
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|record| record.try_get::<String, _>("gateway_identity_key").unwrap())
    .collect::<HashSet<_>>();

    Ok(items)
}

pub(crate) async fn insert_gateway_description(
    conn: &mut DbConnection,
    identity_key: String,
    description: NodeDescriptionResponse,
    timestamp: i64,
) -> anyhow::Result<()> {
    crate::db::query(
        r#"
        INSERT INTO gateway_description (
            gateway_identity_key,
            moniker,
            website,
            security_contact,
            details,
            last_updated_utc
        ) VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT (gateway_identity_key) DO UPDATE SET
            moniker = excluded.moniker,
            website = excluded.website,
            security_contact = excluded.security_contact,
            details = excluded.details,
            last_updated_utc = excluded.last_updated_utc
        "#,
    )
    .bind(identity_key)
    .bind(description.moniker)
    .bind(description.website)
    .bind(description.security_contact)
    .bind(description.details)
    .bind(timestamp)
    .execute(conn.as_mut())
    .await
    .map(drop)
    .map_err(From::from)
}

pub(crate) async fn get_or_create_gateway(
    conn: &mut DbConnection,
    gateway_identity_key: &str,
) -> anyhow::Result<i64> {
    // Try to find existing gateway
    let existing = crate::db::query("SELECT id FROM gateways WHERE gateway_identity_key = ?")
        .bind(gateway_identity_key.to_string())
        .fetch_optional(conn.as_mut())
        .await?;

    if let Some(row) = existing {
        return Ok(row.try_get("id")?);
    }

    // Create new gateway
    tracing::info!("Creating new gateway record for {}", gateway_identity_key);
    let now = crate::utils::now_utc().unix_timestamp();

    let result = crate::db::query(
        r#"INSERT INTO gateways (
            gateway_identity_key, 
            bonded, 
            performance, 
            self_described, 
            last_updated_utc
        ) VALUES (?, ?, ?, ?, ?)
        RETURNING id"#,
    )
    .bind(gateway_identity_key.to_string())
    .bind(true) // Assume bonded since being tested
    .bind(0) // Initial performance
    .bind("null")
    .bind(now)
    .fetch_one(conn.as_mut())
    .await?;

    Ok(result.try_get("id")?)
}
