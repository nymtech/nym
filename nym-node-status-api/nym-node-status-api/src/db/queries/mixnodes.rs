use std::collections::HashSet;

use futures_util::TryStreamExt;
use sqlx::{pool::PoolConnection, Sqlite};
use tracing::error;

use crate::{
    db::{
        models::{MixnodeDto, MixnodeRecord},
        DbPool,
    },
    http::models::{DailyStats, Mixnode},
    mixnet_scraper::helpers::NodeDescriptionResponse,
};

pub(crate) async fn update_mixnodes(
    pool: &DbPool,
    mixnodes: Vec<MixnodeRecord>,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;

    // mark all as unbonded
    sqlx::query!(
        r#"UPDATE
            mixnodes
        SET
            bonded = false
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // existing nodes get updated on insert
    for record in mixnodes.iter() {
        // https://www.sqlite.org/lang_upsert.html
        sqlx::query!(
            "INSERT INTO mixnodes
                (mix_id, identity_key, bonded, total_stake,
                    host, http_api_port, full_details,
                    self_described, last_updated_utc, is_dp_delegatee)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(mix_id) DO UPDATE SET
                bonded=excluded.bonded,
                total_stake=excluded.total_stake, host=excluded.host,
                http_api_port=excluded.http_api_port,
                full_details=excluded.full_details,self_described=excluded.self_described,
                last_updated_utc=excluded.last_updated_utc,
                is_dp_delegatee = excluded.is_dp_delegatee;",
            record.mix_id,
            record.identity_key,
            record.bonded,
            record.total_stake,
            record.host,
            record.http_port,
            record.full_details,
            record.self_described,
            record.last_updated_utc,
            record.is_dp_delegatee
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(())
}

pub(crate) async fn get_all_mixnodes(pool: &DbPool) -> anyhow::Result<Vec<Mixnode>> {
    let mut conn = pool.acquire().await?;
    let items = sqlx::query_as!(
        MixnodeDto,
        r#"SELECT
            mn.mix_id as "mix_id!",
            mn.bonded as "bonded: bool",
            mn.is_dp_delegatee as "is_dp_delegatee: bool",
            mn.total_stake as "total_stake!",
            mn.full_details as "full_details!",
            mn.self_described as "self_described",
            mn.last_updated_utc as "last_updated_utc!",
            COALESCE(md.moniker, "NA") as "moniker!",
            COALESCE(md.website, "NA") as "website!",
            COALESCE(md.security_contact, "NA") as "security_contact!",
            COALESCE(md.details, "NA") as "details!"
         FROM mixnodes mn
         LEFT JOIN mixnode_description md ON mn.mix_id = md.mix_id
         ORDER BY mn.mix_id"#
    )
    .fetch(&mut *conn)
    .try_collect::<Vec<_>>()
    .await?;

    let items = items
        .into_iter()
        .map(|item| item.try_into())
        .collect::<anyhow::Result<Vec<_>>>()
        .map_err(|e| {
            error!("Conversion from DTO failed: {e}. Invalidly stored data?");
            e
        })?;
    Ok(items)
}

pub(crate) async fn get_daily_stats(pool: &DbPool) -> anyhow::Result<Vec<DailyStats>> {
    let mut conn = pool.acquire().await?;
    let items = sqlx::query_as!(
        DailyStats,
        r#"
        SELECT
            date_utc as "date_utc!",
            SUM(total_stake) as "total_stake!: i64",
            SUM(packets_received) as "total_packets_received!: i64",
            SUM(packets_sent) as "total_packets_sent!: i64",
            SUM(packets_dropped) as "total_packets_dropped!: i64"
        FROM (
            SELECT
                date_utc,
                n.total_stake,
                n.packets_received,
                n.packets_sent,
                n.packets_dropped
            FROM nym_node_daily_mixing_stats n
            UNION ALL
            SELECT
                m.date_utc,
                m.total_stake,
                m.packets_received,
                m.packets_sent,
                m.packets_dropped
            FROM mixnode_daily_stats m
            LEFT JOIN nym_node_daily_mixing_stats ON m.mix_id = nym_node_daily_mixing_stats.node_id
            WHERE nym_node_daily_mixing_stats.node_id IS NULL
        )
        GROUP BY date_utc
        ORDER BY date_utc ASC
        "#,
    )
    .fetch(&mut *conn)
    .try_collect::<Vec<DailyStats>>()
    .await?;

    Ok(items)
}

pub(crate) async fn get_bonded_mix_ids(pool: &DbPool) -> anyhow::Result<HashSet<i64>> {
    let mut conn = pool.acquire().await?;
    let items = sqlx::query!(
        r#"
            SELECT mix_id
            FROM mixnodes
            WHERE bonded = true
        "#
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|record| record.mix_id)
    .collect::<HashSet<_>>();

    Ok(items)
}

pub(crate) async fn insert_mixnode_description(
    conn: &mut PoolConnection<Sqlite>,
    mix_id: &i64,
    description: &NodeDescriptionResponse,
    timestamp: i64,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO mixnode_description (
            mix_id, moniker, website, security_contact, details, last_updated_utc
        ) VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT (mix_id) DO UPDATE SET
            moniker = excluded.moniker,
            website = excluded.website,
            security_contact = excluded.security_contact,
            details = excluded.details,
            last_updated_utc = excluded.last_updated_utc
        "#,
        mix_id,
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
