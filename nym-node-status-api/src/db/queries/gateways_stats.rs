use crate::db::{models::GatewaySessionsRecord, DbPool};

pub(crate) async fn insert_session_records(
    pool: &DbPool,
    records: Vec<GatewaySessionsRecord>,
) -> anyhow::Result<()> {
    let mut db = pool.acquire().await?;
    for record in records {
        sqlx::query!(
            "INSERT OR IGNORE INTO gateway_session_stats
                (gateway_identity_key, node_id, date_utc,
                    unique_active_clients, session_started, users_hashes,
                    vpn_sessions, mixnet_sessions, unknown_sessions)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            record.gateway_identity_key,
            record.node_id,
            record.date,
            record.unique_active_clients,
            record.session_started,
            record.users_hashes,
            record.vpn_sessions,
            record.mixnet_sessions,
            record.unknown_sessions,
        )
        .execute(&mut *db)
        .await?;
    }

    Ok(())
}
