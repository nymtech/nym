use crate::db::{
    models::{BondedStatusDto, GatewayRecord},
    DbPool,
};
use futures_util::TryStreamExt;
use nym_validator_client::models::DescribedGateway;

pub(crate) async fn insert_gateways(
    pool: &DbPool,
    gateways: Vec<GatewayRecord>,
) -> anyhow::Result<()> {
    let mut db = pool.acquire().await?;
    for record in gateways {
        let (
            gateway_identity_key,
            bonded,
            blacklisted,
            self_described,
            explorer_pretty_bond,
            last_updated_utc,
            performance,
        ) = record;
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
            gateway_identity_key,
            bonded,
            blacklisted,
            self_described,
            explorer_pretty_bond,
            last_updated_utc,
            performance
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
