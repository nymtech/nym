use crate::node_scraper::models::BridgeInformation;
use crate::{
    db::{
        DbPool,
        models::{InsertNodeScraperRecords, NodeStats, ScrapeNodeKind},
    },
    node_scraper::helpers::update_daily_stats_uncommitted,
    utils::now_utc,
};
use anyhow::Result;
use nym_node_requests::api::v1::gateway::models::SignedBridgeClientParams;
use sqlx::Transaction;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, instrument, warn};

#[instrument(level = "info", skip_all)]
pub(crate) async fn batch_store_node_scraper_results(
    pool: &DbPool,
    results: Arc<Mutex<Vec<InsertNodeScraperRecords>>>,
) -> anyhow::Result<()> {
    let results_iter = results.lock().await;
    info!(
        "📊 ⏳ Storing {} scraped node info (packet stats, bridges, ...) into the DB, {} with signed bridges",
        results_iter.len(),
        results_iter
            .iter()
            .filter(|r| r.bridges_signed.is_some())
            .count()
    );
    let started_at = now_utc();

    let mut tx = pool
        .begin()
        .await
        .map_err(|err| anyhow::anyhow!("Failed to begin transaction: {err}"))?;

    for record in &(*results_iter) {
        if record.bridges.is_some() {
            if let ScrapeNodeKind::EntryExitNymNode {
                node_id,
                identity_key,
            } = &record.node_kind
            {
                insert_node_bridges(
                    &mut tx,
                    *node_id,
                    identity_key,
                    &record.bridges,
                    &record.bridges_signed,
                )
                .await?;
            }
        }

        insert_node_packet_stats_uncommitted(
            &mut tx,
            &record.node_kind,
            &record.stats,
            record.unix_timestamp,
        )
        .await?;

        update_daily_stats_uncommitted(
            &mut tx,
            &record.node_kind,
            record.timestamp_utc,
            &record.stats,
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

async fn insert_node_bridges(
    tx: &mut Transaction<'static, sqlx::Postgres>,
    node_id: i64,
    gateway_identity_key: &str,
    bridges: &Option<BridgeInformation>,
    bridges_signed: &Option<SignedBridgeClientParams>,
) -> Result<()> {
    if let Some(bridges) = bridges {
        let bridges = serde_json::to_value(bridges)?;
        let bridges_signed = bridges_signed
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;
        // a node that signed once is never downgraded to unsigned (an on-path attacker could drop the
        // signed endpoint); an operator who stops signing on purpose resets it with
        // `UPDATE gateways SET bridges_signed = NULL WHERE gateway_identity_key = '<key>'`
        let updated = sqlx::query!(
            r#"
                    UPDATE gateways SET bridges = $1, bridges_signed = $2 WHERE gateway_identity_key = $3 AND (bridges_signed IS NULL OR $2::jsonb IS NOT NULL);
                    "#,
            bridges,
            bridges_signed,
            gateway_identity_key
        )
        .execute(tx.as_mut())
        .await?;
        if updated.rows_affected() == 0 && bridges_signed.is_none() {
            warn!(
                "kept the signed bridge info of node id {node_id}: the scrape returned an unsigned one"
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_crypto::asymmetric::ed25519;

    #[sqlx::test(migrations = "./migrations_pg")]
    async fn signed_row_is_not_downgraded(pool: sqlx::PgPool) {
        let key = ed25519::KeyPair::new(&mut rand::rng());
        let id = key.public_key().to_base58_string();
        sqlx::query(
            "INSERT INTO gateways (gateway_identity_key, self_described, last_updated_utc) VALUES ($1, '', 0)",
        )
        .bind(&id)
        .execute(&pool)
        .await
        .unwrap();
        let payload = r#"{"version": "0", "transports": [], "transports_v2": []}"#;
        let signed = SignedBridgeClientParams::new(payload.to_string(), key.private_key());
        let info: BridgeInformation = serde_json::from_str(payload).unwrap();
        let mut tx = pool.begin().await.unwrap();
        for signed in [Some(signed), None] {
            insert_node_bridges(&mut tx, 1, &id, &Some(info.clone()), &signed)
                .await
                .unwrap();
        }
        tx.commit().await.unwrap();
        let (stored,): (Option<serde_json::Value>,) =
            sqlx::query_as("SELECT bridges_signed FROM gateways WHERE gateway_identity_key = $1")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(stored.is_some());
    }
}
