use crate::{
    db::{models::NodeKind, DbPool},
    mixnet_scraper::helpers::NodeStats,
};
use anyhow::Result;
use chrono::Utc;

pub(crate) async fn insert_node_packet_stats(
    pool: &DbPool,
    node_id: i64,
    node_kind: &NodeKind,
    stats: NodeStats,
) -> Result<()> {
    let timestamp = Utc::now();
    let timestamp_utc = timestamp.timestamp();

    let mut conn = pool.acquire().await?;

    match node_kind {
        NodeKind::LegacyMixnode => {
            sqlx::query!(
                r#"
                INSERT INTO mixnode_packet_stats_raw (
                    mix_id, timestamp_utc, packets_received, packets_sent, packets_dropped
                ) VALUES (?, ?, ?, ?, ?)
                "#,
                node_id,
                timestamp_utc,
                stats.packets_received,
                stats.packets_sent,
                stats.packets_dropped,
            )
            .execute(&mut *conn)
            .await?;
        }
        NodeKind::NymNode => {
            sqlx::query!(
                r#"
                INSERT INTO nym_nodes_packet_stats_raw (
                    node_id, timestamp_utc, packets_received, packets_sent, packets_dropped
                ) VALUES (?, ?, ?, ?, ?)
                "#,
                node_id,
                timestamp_utc,
                stats.packets_received,
                stats.packets_sent,
                stats.packets_dropped,
            )
            .execute(&mut *conn)
            .await?;
        }
    }

    Ok(())
}
