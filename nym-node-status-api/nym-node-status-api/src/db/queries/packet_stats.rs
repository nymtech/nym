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

#[cfg(feature = "sqlite")]
pub(crate) async fn insert_node_packet_stats_uncommitted(
    tx: &mut Transaction<'static, sqlx::Sqlite>,
    node_kind: &ScrapeNodeKind,
    stats: &NodeStats,
    timestamp_utc: i64,
) -> Result<()> {
    match node_kind {
        ScrapeNodeKind::LegacyMixnode { mix_id } => {
            sqlx::query(
                r#"
                INSERT INTO mixnode_packet_stats_raw (
                    mix_id, timestamp_utc, packets_received, packets_sent, packets_dropped
                ) VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(mix_id)
            .bind(timestamp_utc)
            .bind(stats.packets_received)
            .bind(stats.packets_sent)
            .bind(stats.packets_dropped)
            .execute(tx.as_mut())
            .await?;
        }
        ScrapeNodeKind::MixingNymNode { node_id }
        | ScrapeNodeKind::EntryExitNymNode { node_id, .. } => {
            sqlx::query(
                r#"
                INSERT INTO nym_nodes_packet_stats_raw (
                    node_id, timestamp_utc, packets_received, packets_sent, packets_dropped
                ) VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(node_id)
            .bind(timestamp_utc)
            .bind(stats.packets_received)
            .bind(stats.packets_sent)
            .bind(stats.packets_dropped)
            .execute(tx.as_mut())
            .await?;
        }
    }

    Ok(())
}

#[cfg(feature = "pg")]
pub(crate) async fn insert_node_packet_stats_uncommitted(
    tx: &mut Transaction<'static, sqlx::Postgres>,
    node_kind: &ScrapeNodeKind,
    stats: &NodeStats,
    timestamp_utc: i64,
) -> Result<()> {
    match node_kind {
        ScrapeNodeKind::LegacyMixnode { mix_id } => {
            sqlx::query(
                r#"
                INSERT INTO mixnode_packet_stats_raw (
                    mix_id, timestamp_utc, packets_received, packets_sent, packets_dropped
                ) VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(mix_id)
            .bind(timestamp_utc)
            .bind(stats.packets_received)
            .bind(stats.packets_sent)
            .bind(stats.packets_dropped)
            .execute(tx.as_mut())
            .await?;
        }
        ScrapeNodeKind::MixingNymNode { node_id }
        | ScrapeNodeKind::EntryExitNymNode { node_id, .. } => {
            sqlx::query(
                r#"
                INSERT INTO nym_nodes_packet_stats_raw (
                    node_id, timestamp_utc, packets_received, packets_sent, packets_dropped
                ) VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(node_id)
            .bind(timestamp_utc)
            .bind(stats.packets_received)
            .bind(stats.packets_sent)
            .bind(stats.packets_dropped)
            .execute(tx.as_mut())
            .await?;
        }
    }

    Ok(())
}

#[cfg(feature = "sqlite")]
pub(crate) async fn get_raw_node_stats(
    tx: &mut Transaction<'static, sqlx::Sqlite>,
    node_kind: &ScrapeNodeKind,
) -> Result<Option<NodeStats>> {
    let packets = match node_kind {
        // if no packets are found, it's fine to assume 0 because that's also
        // SQL default value if none provided
        ScrapeNodeKind::LegacyMixnode { mix_id } => {
            sqlx::query_as::<_, NodeStats>(
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
            )
            .bind(mix_id)
            .fetch_optional(tx.as_mut())
            .await?
        }
        ScrapeNodeKind::MixingNymNode { node_id }
        | ScrapeNodeKind::EntryExitNymNode { node_id, .. } => {
            sqlx::query_as::<_, NodeStats>(
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
            )
            .bind(node_id)
            .fetch_optional(tx.as_mut())
            .await?
        }
    };

    Ok(packets)
}

#[cfg(feature = "pg")]
pub(crate) async fn get_raw_node_stats(
    tx: &mut Transaction<'static, sqlx::Postgres>,
    node_kind: &ScrapeNodeKind,
) -> Result<Option<NodeStats>> {
    let packets = match node_kind {
        // if no packets are found, it's fine to assume 0 because that's also
        // SQL default value if none provided
        ScrapeNodeKind::LegacyMixnode { mix_id } => {
            sqlx::query_as::<_, NodeStats>(
                r#"
                SELECT
                    COALESCE(packets_received, 0) as packets_received,
                    COALESCE(packets_sent, 0) as packets_sent,
                    COALESCE(packets_dropped, 0) as packets_dropped
                FROM mixnode_packet_stats_raw
                WHERE mix_id = $1
                ORDER BY timestamp_utc DESC
                LIMIT 1 OFFSET 1
                "#,
            )
            .bind(mix_id)
            .fetch_optional(tx.as_mut())
            .await?
        }
        ScrapeNodeKind::MixingNymNode { node_id }
        | ScrapeNodeKind::EntryExitNymNode { node_id, .. } => {
            sqlx::query_as::<_, NodeStats>(
                r#"
                SELECT
                    COALESCE(packets_received, 0) as packets_received,
                    COALESCE(packets_sent, 0) as packets_sent,
                    COALESCE(packets_dropped, 0) as packets_dropped
                FROM nym_nodes_packet_stats_raw
                WHERE node_id = $1
                ORDER BY timestamp_utc DESC
                LIMIT 1 OFFSET 1
                "#,
            )
            .bind(node_id)
            .fetch_optional(tx.as_mut())
            .await?
        }
    };

    Ok(packets)
}

#[cfg(feature = "sqlite")]
pub(crate) async fn insert_daily_node_stats_uncommitted(
    tx: &mut Transaction<'static, sqlx::Sqlite>,
    node_kind: &ScrapeNodeKind,
    date_utc: &str,
    packets: NodeStats,
) -> Result<()> {
    match node_kind {
        ScrapeNodeKind::LegacyMixnode { mix_id } => {
            let total_stake = sqlx::query_scalar::<_, i64>(
                r#"
                    SELECT
                        total_stake
                    FROM mixnodes
                    WHERE mix_id = ?
                   "#,
            )
            .bind(mix_id)
            .fetch_one(tx.as_mut())
            .await?;

            sqlx::query(
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
            )
            .bind(mix_id)
            .bind(date_utc)
            .bind(total_stake)
            .bind(packets.packets_received)
            .bind(packets.packets_sent)
            .bind(packets.packets_dropped)
            .execute(tx.as_mut())
            .await?;
        }
        ScrapeNodeKind::MixingNymNode { node_id }
        | ScrapeNodeKind::EntryExitNymNode { node_id, .. } => {
            let total_stake = sqlx::query_scalar::<_, i64>(
                r#"
                SELECT
                    total_stake
                FROM nym_nodes
                WHERE node_id = ?
                "#,
            )
            .bind(node_id)
            .fetch_one(tx.as_mut())
            .await?;

            sqlx::query(
                r#"
                INSERT INTO nym_node_daily_mixing_stats (
                    node_id, date_utc,
                    total_stake, packets_received,
                    packets_sent, packets_dropped
                ) VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT(node_id, date_utc) DO UPDATE SET
                    total_stake = excluded.total_stake,
                    packets_received = nym_node_daily_mixing_stats.packets_received + excluded.packets_received,
                    packets_sent = nym_node_daily_mixing_stats.packets_sent + excluded.packets_sent,
                    packets_dropped = nym_node_daily_mixing_stats.packets_dropped + excluded.packets_dropped
                "#,
            )
            .bind(node_id)
            .bind(date_utc)
            .bind(total_stake)
            .bind(packets.packets_received)
            .bind(packets.packets_sent)
            .bind(packets.packets_dropped)
            .execute(tx.as_mut())
            .await?;
        }
    }

    Ok(())
}

#[cfg(feature = "pg")]
pub(crate) async fn insert_daily_node_stats_uncommitted(
    tx: &mut Transaction<'static, sqlx::Postgres>,
    node_kind: &ScrapeNodeKind,
    date_utc: &str,
    packets: NodeStats,
) -> Result<()> {
    match node_kind {
        ScrapeNodeKind::LegacyMixnode { mix_id } => {
            let total_stake = sqlx::query_scalar::<_, i64>(
                r#"
                    SELECT
                        total_stake
                    FROM mixnodes
                    WHERE mix_id = $1
                   "#,
            )
            .bind(mix_id)
            .fetch_one(tx.as_mut())
            .await?;

            sqlx::query(
                r#"
                INSERT INTO mixnode_daily_stats (
                    mix_id, date_utc,
                    total_stake, packets_received,
                    packets_sent, packets_dropped
                ) VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT(mix_id, date_utc) DO UPDATE SET
                    total_stake = excluded.total_stake,
                    packets_received = mixnode_daily_stats.packets_received + excluded.packets_received,
                    packets_sent = mixnode_daily_stats.packets_sent + excluded.packets_sent,
                    packets_dropped = mixnode_daily_stats.packets_dropped + excluded.packets_dropped
                "#,
            )
            .bind(mix_id)
            .bind(date_utc)
            .bind(total_stake)
            .bind(packets.packets_received)
            .bind(packets.packets_sent)
            .bind(packets.packets_dropped)
            .execute(tx.as_mut())
            .await?;
        }
        ScrapeNodeKind::MixingNymNode { node_id }
        | ScrapeNodeKind::EntryExitNymNode { node_id, .. } => {
            let total_stake = sqlx::query_scalar::<_, i64>(
                r#"
                SELECT
                    total_stake
                FROM nym_nodes
                WHERE node_id = $1
                "#,
            )
            .bind(node_id)
            .fetch_one(tx.as_mut())
            .await?;

            sqlx::query(
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
            )
            .bind(node_id)
            .bind(date_utc)
            .bind(total_stake)
            .bind(packets.packets_received)
            .bind(packets.packets_sent)
            .bind(packets.packets_dropped)
            .execute(tx.as_mut())
            .await?;
        }
    }

    Ok(())
}
