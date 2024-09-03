use crate::db::{models::MixnodeRecord, DbPool};

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
