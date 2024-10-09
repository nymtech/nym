use futures_util::TryStreamExt;
use nym_validator_client::models::MixNodeBondAnnotated;
use tracing::error;

use crate::{
    db::{
        models::{BondedStatusDto, MixnodeDto, MixnodeRecord},
        DbPool,
    },
    http::models::{DailyStats, Mixnode},
};

pub(crate) async fn insert_mixnodes(
    pool: &DbPool,
    mixnodes: Vec<MixnodeRecord>,
) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;

    for record in mixnodes.iter() {
        // https://www.sqlite.org/lang_upsert.html
        sqlx::query!(
            "INSERT INTO mixnodes
                (mix_id, identity_key, bonded, total_stake,
                    host, http_api_port, blacklisted, full_details,
                    self_described, last_updated_utc, is_dp_delegatee)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(mix_id) DO UPDATE SET
                bonded=excluded.bonded,
                total_stake=excluded.total_stake, host=excluded.host,
                http_api_port=excluded.http_api_port,blacklisted=excluded.blacklisted,
                full_details=excluded.full_details,self_described=excluded.self_described,
                last_updated_utc=excluded.last_updated_utc,
                is_dp_delegatee = excluded.is_dp_delegatee;",
            record.mix_id,
            record.identity_key,
            record.bonded,
            record.total_stake,
            record.host,
            record.http_port,
            record.blacklisted,
            record.full_details,
            record.self_described,
            record.last_updated_utc,
            record.is_dp_delegatee
        )
        .execute(&mut *conn)
        .await?;
    }

    Ok(())
}

pub(crate) async fn get_all_mixnodes(pool: &DbPool) -> anyhow::Result<Vec<Mixnode>> {
    let mut conn = pool.acquire().await?;
    let items = sqlx::query_as!(
        MixnodeDto,
        r#"SELECT
            mn.mix_id as "mix_id!",
            mn.bonded as "bonded: bool",
            mn.blacklisted as "blacklisted: bool",
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
    .fetch(&mut conn)
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

/// We fetch the latest 30 days of data as a subquery and then
/// return it in ascending order, so we don't break existing UI
pub(crate) async fn get_daily_stats(pool: &DbPool) -> anyhow::Result<Vec<DailyStats>> {
    let mut conn = pool.acquire().await?;
    let items = sqlx::query_as!(
        DailyStats,
        r#"
        SELECT
            date_utc as "date_utc!",
            packets_received as "total_packets_received!: i64",
            packets_sent as "total_packets_sent!: i64",
            packets_dropped as "total_packets_dropped!: i64",
            total_stake as "total_stake!: i64"
        FROM (
            SELECT
                date_utc,
                SUM(packets_received) as packets_received,
                SUM(packets_sent) as packets_sent,
                SUM(packets_dropped) as packets_dropped,
                SUM(total_stake) as total_stake
            FROM mixnode_daily_stats
            GROUP BY date_utc
            ORDER BY date_utc DESC
            LIMIT 30
        )
        GROUP BY date_utc
        ORDER BY date_utc
        "#
    )
    .fetch(&mut conn)
    .try_collect::<Vec<DailyStats>>()
    .await?;

    Ok(items)
}

/// Ensure all mixnodes that are set as bonded, are still bonded
pub(crate) async fn ensure_mixnodes_still_bonded(
    pool: &DbPool,
    mixnodes: &[MixNodeBondAnnotated],
) -> anyhow::Result<usize> {
    let bonded_mixnodes_rows = get_all_bonded_mixnodes_row_ids_by_status(pool, true).await?;
    let unbonded_mixnodes_rows = bonded_mixnodes_rows.iter().filter(|v| {
        !mixnodes
            .iter()
            .any(|bonded| *bonded.mixnode_details.bond_information.identity() == v.identity_key)
    });

    let recently_unbonded_mixnodes = unbonded_mixnodes_rows.to_owned().count();
    let last_updated_utc = chrono::offset::Utc::now().timestamp();
    let mut transaction = pool.begin().await?;
    for row in unbonded_mixnodes_rows {
        sqlx::query!(
            "UPDATE mixnodes
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

    Ok(recently_unbonded_mixnodes)
}

async fn get_all_bonded_mixnodes_row_ids_by_status(
    pool: &DbPool,
    status: bool,
) -> anyhow::Result<Vec<BondedStatusDto>> {
    let mut conn = pool.acquire().await?;
    let items = sqlx::query_as!(
        BondedStatusDto,
        r#"SELECT
            id as "id!",
            identity_key as "identity_key!",
            bonded as "bonded: bool"
         FROM mixnodes
         WHERE bonded = ?"#,
        status,
    )
    .fetch(&mut *conn)
    .try_collect::<Vec<_>>()
    .await?;

    Ok(items)
}
