use crate::{
    db::{models::GatewaySessionsRecord, DbPool},
    http::models::SessionStats,
};
use futures_util::TryStreamExt;
use time::Date;
use tracing::error;

#[cfg(feature = "sqlite")]
pub(crate) async fn insert_session_records(
    pool: &DbPool,
    records: Vec<GatewaySessionsRecord>,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    for record in records {
        sqlx::query!(
            "INSERT OR IGNORE INTO gateway_session_stats
                (gateway_identity_key, node_id, day,
                    unique_active_clients, session_started, users_hashes,
                    vpn_sessions, mixnet_sessions, unknown_sessions)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            record.gateway_identity_key,
            record.node_id,
            record.day,
            record.unique_active_clients,
            record.session_started,
            record.users_hashes,
            record.vpn_sessions,
            record.mixnet_sessions,
            record.unknown_sessions,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(())
}

#[cfg(feature = "pg")]
pub(crate) async fn insert_session_records(
    pool: &DbPool,
    records: Vec<GatewaySessionsRecord>,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    for record in records {
        sqlx::query!(
            "INSERT INTO gateway_session_stats
                (gateway_identity_key, node_id, day,
                    unique_active_clients, session_started, users_hashes,
                    vpn_sessions, mixnet_sessions, unknown_sessions)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT DO NOTHING",
            record.gateway_identity_key,
            record.node_id,
            record.day,
            record.unique_active_clients,
            record.session_started,
            record.users_hashes,
            record.vpn_sessions,
            record.mixnet_sessions,
            record.unknown_sessions,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(())
}

pub(crate) async fn get_sessions_stats(pool: &DbPool) -> anyhow::Result<Vec<SessionStats>> {
    let mut conn = pool.acquire().await?;
    let items = sqlx::query_as(
        "SELECT gateway_identity_key, 
                node_id,              
                day,                  
                unique_active_clients,
                session_started,      
                users_hashes,         
                vpn_sessions,         
                mixnet_sessions,      
                unknown_sessions
        FROM gateway_session_stats",
    )
    .fetch(&mut *conn)
    .try_collect::<Vec<GatewaySessionsRecord>>()
    .await?;

    let items: Vec<SessionStats> = items
        .into_iter()
        .map(|item| item.try_into())
        .collect::<anyhow::Result<Vec<_>>>()
        .map_err(|e| {
            error!("Conversion from database failed: {e}. Invalidly stored data?");
            e
        })?;

    Ok(items)
}

pub(crate) async fn delete_old_records(pool: &DbPool, cut_off: Date) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;
    sqlx::query!("DELETE FROM gateway_session_stats WHERE day <= ?", cut_off)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
