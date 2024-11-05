use crate::{
    db::{
        models::{BondedStatusDto, GatewayDto, GatewayRecord},
        DbPool,
    },
    http::models::Gateway,
};
use futures_util::TryStreamExt;
use nym_validator_client::models::DescribedGateway;
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
    gateways: Vec<GatewayRecord>,
) -> anyhow::Result<()> {
    let mut db = pool.acquire().await?;
    for record in gateways {
        sqlx::query!(
            "INSERT INTO gateways
                (gateway_identity_key, bonded, blacklisted,
                    self_described, explorer_pretty_bond,
                    last_updated_utc, performance)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(gateway_identity_key) DO UPDATE SET
                bonded=excluded.bonded,
                blacklisted=excluded.blacklisted,
                self_described=excluded.self_described,
                explorer_pretty_bond=excluded.explorer_pretty_bond,
                last_updated_utc=excluded.last_updated_utc,
                performance = excluded.performance;",
            record.identity_key,
            record.bonded,
            record.blacklisted,
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

pub(crate) async fn write_blacklisted_gateways_to_db<'a, I>(
    pool: &DbPool,
    gateways: I,
) -> anyhow::Result<()>
where
    I: Iterator<Item = &'a String>,
{
    let mut conn = pool.acquire().await?;
    for gateway_identity_key in gateways {
        sqlx::query!(
            "UPDATE gateways
             SET blacklisted = true
             WHERE gateway_identity_key = ?;",
            gateway_identity_key,
        )
        .execute(&mut *conn)
        .await?;
    }

    Ok(())
}

/// Ensure all gateways that are set as bonded, are still bonded
pub(crate) async fn ensure_gateways_still_bonded(
    pool: &DbPool,
    gateways: &[DescribedGateway],
) -> anyhow::Result<usize> {
    let bonded_gateways_rows = get_all_bonded_gateways_row_ids_by_status(pool, true).await?;
    let unbonded_gateways_rows = bonded_gateways_rows.iter().filter(|v| {
        !gateways
            .iter()
            .any(|bonded| *bonded.bond.identity() == v.identity_key)
    });

    let recently_unbonded_gateways = unbonded_gateways_rows.to_owned().count();
    let last_updated_utc = chrono::offset::Utc::now().timestamp();
    let mut transaction = pool.begin().await?;
    for row in unbonded_gateways_rows {
        sqlx::query!(
            "UPDATE gateways
                SET bonded = ?, last_updated_utc = ?
                WHERE id = ?;",
            false,
            last_updated_utc,
            row.id,
        )
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;

    Ok(recently_unbonded_gateways)
}

async fn get_all_bonded_gateways_row_ids_by_status(
    pool: &DbPool,
    status: bool,
) -> anyhow::Result<Vec<BondedStatusDto>> {
    let mut conn = pool.acquire().await?;
    let items = sqlx::query_as!(
        BondedStatusDto,
        r#"SELECT
            id as "id!",
            gateway_identity_key as "identity_key!",
            bonded as "bonded: bool"
         FROM gateways
         WHERE bonded = ?"#,
        status,
    )
    .fetch(&mut *conn)
    .try_collect::<Vec<_>>()
    .await?;

    Ok(items)
}

pub(crate) async fn get_all_gateways(pool: &DbPool) -> anyhow::Result<Vec<Gateway>> {
    let mut conn = pool.acquire().await?;
    let items = sqlx::query_as!(
        GatewayDto,
        r#"SELECT
            gw.gateway_identity_key as "gateway_identity_key!",
            gw.bonded as "bonded: bool",
            gw.blacklisted as "blacklisted: bool",
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
