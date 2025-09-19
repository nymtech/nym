use crate::{
    db::{
        models::{InsertStatsRecord, NodeStats, ScrapeNodeKind},
        DbPool,
    },
    node_scraper::helpers::update_daily_stats_uncommitted,
    utils::now_utc,
};
use anyhow::Result;
use sqlx::Transaction;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, instrument};

#[instrument(level = "info", skip_all)]
pub(crate) async fn batch_store_packet_stats(
    pool: &DbPool,
    results: Arc<Mutex<Vec<InsertStatsRecord>>>,
) -> anyhow::Result<()> {
    let results_iter = results.lock().await;
    info!(
        "📊 ⏳ Storing {} packet stats into the DB",
        results_iter.len()
    );
    let started_at = now_utc();

    let mut tx = pool
        .begin()
        .await
        .map_err(|err| anyhow::anyhow!("Failed to begin transaction: {err}"))?;

    for stats_record in &(*results_iter) {
        insert_node_packet_stats_uncommitted(
            &mut tx,
            &stats_record.node_kind,
            &stats_record.stats,
            stats_record.unix_timestamp,
        )
        .await?;

        update_daily_stats_uncommitted(
            &mut tx,
            &stats_record.node_kind,
            stats_record.timestamp_utc,
            &stats_record.stats,
        )
        .await?;
    }

    tx.commit()
        .await
        .inspect(|_| {
            let elapsed = now_utc() - started_at;
            info!(
                "📊 ☑️ Packet stats successfully committed to DB (took {}s)",
                elapsed.as_seconds_f32()
            );
        })
        .map_err(|err| anyhow::anyhow!("Failed to commit: {err}"))
}

async fn insert_node_packet_stats_uncommitted(
    tx: &mut Transaction<'static, sqlx::Postgres>,
    node_kind: &ScrapeNodeKind,
    stats: &NodeStats,
    timestamp_utc: i64,
) -> Result<()> {
    match node_kind {
        ScrapeNodeKind::MixingNymNode { node_id }
        | ScrapeNodeKind::EntryExitNymNode { node_id, .. } => {
            sqlx::query!(
                r#"
                INSERT INTO nym_nodes_packet_stats_raw (
                    node_id, timestamp_utc, packets_received, packets_sent, packets_dropped
                ) VALUES ($1, $2, $3, $4, $5)
                "#,
                *node_id as i32,
                timestamp_utc as i32,
                stats.packets_received,
                stats.packets_sent,
                stats.packets_dropped,
            )
            .execute(tx.as_mut())
            .await?;
        }
    }

    Ok(())
}

pub(crate) async fn get_raw_node_stats(
    tx: &mut Transaction<'static, sqlx::Postgres>,
    node_kind: &ScrapeNodeKind,
) -> Result<Option<NodeStats>> {
    let packets = match node_kind {
        // if no packets are found, it's fine to assume 0 because that's also
        // SQL default value if none provided
        ScrapeNodeKind::MixingNymNode { node_id }
        | ScrapeNodeKind::EntryExitNymNode { node_id, .. } => {
            sqlx::query_as!(
                NodeStats,
                r#"
                SELECT
                    COALESCE(packets_received, 0) as "packets_received!: _",
                    COALESCE(packets_sent, 0) as "packets_sent!: _",
                    COALESCE(packets_dropped, 0) as "packets_dropped!: _"
                FROM nym_nodes_packet_stats_raw
                WHERE node_id = $1
                ORDER BY timestamp_utc DESC
                LIMIT 1 OFFSET 1
                "#,
                *node_id as i32,
            )
            .fetch_optional(tx.as_mut())
            .await?
        }
    };

    Ok(packets)
}

pub(crate) async fn insert_daily_node_stats_uncommitted(
    tx: &mut Transaction<'static, sqlx::Postgres>,
    node_kind: &ScrapeNodeKind,
    date_utc: &str,
    packets: NodeStats,
) -> Result<()> {
    match node_kind {
        ScrapeNodeKind::MixingNymNode { node_id }
        | ScrapeNodeKind::EntryExitNymNode { node_id, .. } => {
            let total_stake = sqlx::query_scalar!(
                r#"
                SELECT
                    total_stake
                FROM nym_nodes
                WHERE node_id = $1
                "#,
                *node_id as i32
            )
            .fetch_one(tx.as_mut())
            .await?;

            sqlx::query!(
                r#"
                INSERT INTO nym_node_daily_mixing_stats (
                    node_id, date_utc,
                    total_stake, packets_received,
                    packets_sent, packets_dropped
                ) VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT(node_id, date_utc) DO UPDATE SET
                    total_stake = excluded.total_stake,
                    packets_received = nym_node_daily_mixing_stats.packets_received + excluded.packets_received,
                    packets_sent = nym_node_daily_mixing_stats.packets_sent + excluded.packets_sent,
                    packets_dropped = nym_node_daily_mixing_stats.packets_dropped + excluded.packets_dropped
                "#,
                *node_id as i32,
                date_utc,
                total_stake,
                packets.packets_received,
                packets.packets_sent,
                packets.packets_dropped,
            )
            .execute(tx.as_mut())
            .await?;
        }
    }

    Ok(())
}
