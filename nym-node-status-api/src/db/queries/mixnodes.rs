use futures_util::TryStreamExt;
use nym_validator_client::models::MixNodeBondAnnotated;

use crate::db::{
    models::{BondedStatusDto, MixnodeRecord},
    DbPool,
};

pub(crate) async fn insert_mixnodes(
    pool: &DbPool,
    mixnodes: Vec<MixnodeRecord>,
) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;

    for record in mixnodes.iter() {
        let (
            mix_id,
            identity_key,
            bonded,
            total_stake,
            host,
            http_port,
            blacklisted,
            full_details,
            self_described,
            last_updated_utc,
            is_dp_delegatee,
        ) = record;
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
            mix_id,
            identity_key,
            bonded,
            total_stake,
            host,
            http_port,
            blacklisted,
            full_details,
            self_described,
            last_updated_utc,
            is_dp_delegatee
        )
        .execute(&mut *conn)
        .await?;
    }

    Ok(())
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
