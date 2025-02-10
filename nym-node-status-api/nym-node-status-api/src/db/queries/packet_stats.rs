use crate::db::{
    models::{MixingNodeKind, NodeStats, ScraperNodeInfo},
    DbPool,
};
use anyhow::Result;

pub(crate) async fn insert_node_packet_stats(
    pool: &DbPool,
    node_id: i64,
    node_kind: &MixingNodeKind,
    stats: &NodeStats,
    timestamp_utc: i64,
) -> Result<()> {
    let mut conn = pool.acquire().await?;

    match node_kind {
        MixingNodeKind::LegacyMixnode => {
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
        MixingNodeKind::NymNode => {
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

pub(crate) async fn get_raw_node_stats(
    pool: &DbPool,
    node: &ScraperNodeInfo,
) -> Result<Option<NodeStats>> {
    let mut conn = pool.acquire().await?;

    let packets = match node.node_kind {
        // if no packets are found, it's fine to assume 0 because that's also
        // SQL default value if none provided
        MixingNodeKind::LegacyMixnode => {
            sqlx::query_as!(
                NodeStats,
                r#"
                SELECT
                    COALESCE(packets_received, 0) as packets_received,
                    COALESCE(packets_sent, 0) as packets_sent,
                    COALESCE(packets_dropped, 0) as packets_dropped
                FROM mixnode_packet_stats_raw
                WHERE mix_id = ?
                ORDER BY timestamp_utc DESC
                LIMIT 1 OFFSET 1
                "#,
                node.node_id
            )
            .fetch_optional(&mut *conn)
            .await?
        }
        MixingNodeKind::NymNode => {
            sqlx::query_as!(
                NodeStats,
                r#"
                SELECT
                    COALESCE(packets_received, 0) as packets_received,
                    COALESCE(packets_sent, 0) as packets_sent,
                    COALESCE(packets_dropped, 0) as packets_dropped
                FROM nym_nodes_packet_stats_raw
                WHERE node_id = ?
                ORDER BY timestamp_utc DESC
                LIMIT 1 OFFSET 1
                "#,
                node.node_id
            )
            .fetch_optional(&mut *conn)
            .await?
        }
    };

    Ok(packets)
}

pub(crate) async fn insert_daily_node_stats(
    pool: &DbPool,
    node: &ScraperNodeInfo,
    date_utc: &str,
    packets: NodeStats,
) -> Result<()> {
    let mut conn = pool.acquire().await?;

    match node.node_kind {
        MixingNodeKind::LegacyMixnode => {
            let total_stake = sqlx::query_scalar!(
                r#"
                    SELECT
                        total_stake
                    FROM mixnodes
                    WHERE mix_id = ?
                   "#,
                node.node_id
            )
            .fetch_one(&mut *conn)
            .await?;

            sqlx::query!(
                r#"
                INSERT INTO mixnode_daily_stats (
                    mix_id, date_utc,
                    total_stake, packets_received,
                    packets_sent, packets_dropped
                ) VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT(mix_id, date_utc) DO UPDATE SET
                    total_stake = excluded.total_stake,
                    packets_received = mixnode_daily_stats.packets_received + excluded.packets_received,
                    packets_sent = mixnode_daily_stats.packets_sent + excluded.packets_sent,
                    packets_dropped = mixnode_daily_stats.packets_dropped + excluded.packets_dropped
                "#,
                node.node_id,
                date_utc,
                total_stake,
                packets.packets_received,
                packets.packets_sent,
                packets.packets_dropped,
            )
            .execute(&mut *conn)
            .await?;
        }
        MixingNodeKind::NymNode => {
            let total_stake = sqlx::query_scalar!(
                r#"
                SELECT
                    total_stake
                FROM nym_nodes
                WHERE node_id = ?
                "#,
                node.node_id
            )
            .fetch_one(&mut *conn)
            .await?;

            sqlx::query!(
                r#"
                INSERT INTO nym_node_daily_mixing_stats (
                    node_id, date_utc,
                    total_stake, packets_received,
                    packets_sent, packets_dropped
                ) VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT(node_id, date_utc) DO UPDATE SET
                    total_stake = nym_node_daily_mixing_stats.total_stake + excluded.total_stake,
                    packets_received = nym_node_daily_mixing_stats.packets_received + excluded.packets_received,
                    packets_sent = nym_node_daily_mixing_stats.packets_sent + excluded.packets_sent,
                    packets_dropped = nym_node_daily_mixing_stats.packets_dropped + excluded.packets_dropped
                "#,
                node.node_id,
                date_utc,
                total_stake,
                packets.packets_received,
                packets.packets_sent,
                packets.packets_dropped,
            )
            .execute(&mut *conn)
            .await?;
        }
    }

    Ok(())
}
