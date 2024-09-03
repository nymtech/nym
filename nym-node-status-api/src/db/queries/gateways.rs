use crate::db::{models::GatewayRecord, DbPool};
use nym_validator_client::client::IdentityKey;

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

pub(crate) async fn write_blacklisted_gateways_to_db(
    pool: &DbPool,
    gateways: Vec<IdentityKey>,
) -> anyhow::Result<()> {
    let mut db = pool.acquire().await?;
    for gateway_identity_key in gateways {
        sqlx::query!(
            "UPDATE gateways
             SET blacklisted = true
             WHERE gateway_identity_key = ?;",
            gateway_identity_key,
        )
        .execute(&mut *db)
        .await?;
    }

    Ok(())
}
