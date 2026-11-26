use crate::node_scraper::models::BridgeScrape;
use crate::{
    db::{
        DbPool,
        models::{InsertNodeScraperRecords, NodeStats, ScrapeNodeKind},
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
pub(crate) async fn batch_store_node_scraper_results(
    pool: &DbPool,
    results: Arc<Mutex<Vec<InsertNodeScraperRecords>>>,
) -> anyhow::Result<()> {
    let results_iter = results.lock().await;
    info!(
        "📊 ⏳ Storing {} scraped node info (packet stats, bridges, ...) into the DB",
        results_iter.len()
    );
    let started_at = now_utc();

    let mut tx = pool
        .begin()
        .await
        .map_err(|err| anyhow::anyhow!("Failed to begin transaction: {err}"))?;

    for record in &(*results_iter) {
        if let ScrapeNodeKind::EntryExitNymNode {
            node_id: _,
            identity_key,
        } = &record.node_kind
        {
            apply_bridge_scrape(&mut tx, identity_key, &record.bridges).await?;
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

async fn apply_bridge_scrape(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    gateway_identity_key: &str,
    bridges: &BridgeScrape,
) -> Result<()> {
    match bridges {
        BridgeScrape::Found(info) => {
            let bridges = serde_json::to_value(info)?;
            // `bonded = true`: an in-flight scrape must not resurrect bridges on a
            // row the monitor has already marked unbonded and cleared.
            sqlx::query!(
                r#"
                    UPDATE gateways SET bridges = $1 WHERE gateway_identity_key = $2 AND bonded = true;
                    "#,
                bridges,
                gateway_identity_key
            )
            .execute(tx.as_mut())
            .await?;
        }
        BridgeScrape::NotPublished => {
            // Definitive absence (404 after successful /stats): drop stale details.
            sqlx::query!(
                r#"
                    UPDATE gateways
                    SET bridges = NULL
                    WHERE gateway_identity_key = $1
                      AND bonded = true
                      AND bridges IS NOT NULL
                    "#,
                gateway_identity_key
            )
            .execute(tx.as_mut())
            .await?;
        }
        BridgeScrape::Unchecked => {}
    }

    Ok(())
}

#[cfg(test)]
mod bridge_scrape_db_tests {
    use super::*;
    use crate::node_scraper::models::BridgeInformation;
    use sqlx::PgPool;

    async fn pool_or_skip() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let pool = PgPool::connect(&url).await.ok()?;
        // Fresh compose volumes have no schema; apply the same migrator as Storage::init.
        sqlx::migrate!("./migrations_pg").run(&pool).await.ok()?;
        Some(pool)
    }

    async fn insert_gateway(
        pool: &PgPool,
        identity: &str,
        bonded: bool,
        bridges: Option<serde_json::Value>,
    ) {
        sqlx::query(
            r#"
            INSERT INTO gateways (
                gateway_identity_key, self_described, last_updated_utc, bonded, bridges
            ) VALUES ($1, '{}', 0, $2, $3)
            ON CONFLICT (gateway_identity_key) DO UPDATE SET
                bonded = EXCLUDED.bonded,
                bridges = EXCLUDED.bridges
            "#,
        )
        .bind(identity)
        .bind(bonded)
        .bind(bridges)
        .execute(pool)
        .await
        .expect("insert gateway");
    }

    async fn read_bridges(pool: &PgPool, identity: &str) -> Option<serde_json::Value> {
        sqlx::query_scalar::<_, Option<serde_json::Value>>(
            "SELECT bridges FROM gateways WHERE gateway_identity_key = $1",
        )
        .bind(identity)
        .fetch_one(pool)
        .await
        .expect("read bridges")
    }

    fn sample_info() -> BridgeInformation {
        BridgeInformation {
            version: "0".into(),
            transports: Vec::new(),
            transports_v2: Vec::new(),
        }
    }

    #[tokio::test]
    async fn found_writes_bridges_on_bonded_gateway() {
        let Some(pool) = pool_or_skip().await else {
            eprintln!("skipping: DATABASE_URL not set / unreachable");
            return;
        };
        let id = "test-bridge-found-bonded";
        insert_gateway(&pool, id, true, None).await;

        let mut tx = pool.begin().await.unwrap();
        apply_bridge_scrape(&mut tx, id, &BridgeScrape::Found(sample_info()))
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let stored = read_bridges(&pool, id).await;
        assert!(stored.is_some(), "Found must write bridges on bonded row");
    }

    #[tokio::test]
    async fn found_does_not_resurrect_bridges_on_unbonded_gateway() {
        let Some(pool) = pool_or_skip().await else {
            eprintln!("skipping: DATABASE_URL not set / unreachable");
            return;
        };
        let id = "test-bridge-found-unbonded";
        insert_gateway(&pool, id, false, None).await;

        let mut tx = pool.begin().await.unwrap();
        apply_bridge_scrape(&mut tx, id, &BridgeScrape::Found(sample_info()))
            .await
            .unwrap();
        tx.commit().await.unwrap();

        assert!(
            read_bridges(&pool, id).await.is_none(),
            "Found must not write bridges when bonded = false"
        );
    }

    #[tokio::test]
    async fn not_published_clears_bonded_bridges() {
        let Some(pool) = pool_or_skip().await else {
            eprintln!("skipping: DATABASE_URL not set / unreachable");
            return;
        };
        let id = "test-bridge-not-published";
        let prior = serde_json::json!({"version": "0", "transports": []});
        insert_gateway(&pool, id, true, Some(prior)).await;

        let mut tx = pool.begin().await.unwrap();
        apply_bridge_scrape(&mut tx, id, &BridgeScrape::NotPublished)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        assert!(
            read_bridges(&pool, id).await.is_none(),
            "NotPublished must NULL bridges on bonded row"
        );
    }

    #[tokio::test]
    async fn unchecked_leaves_bridges_alone() {
        let Some(pool) = pool_or_skip().await else {
            eprintln!("skipping: DATABASE_URL not set / unreachable");
            return;
        };
        let id = "test-bridge-unchecked";
        let prior = serde_json::json!({"version": "0", "transports": [{"keep": true}]});
        insert_gateway(&pool, id, true, Some(prior.clone())).await;

        let mut tx = pool.begin().await.unwrap();
        apply_bridge_scrape(&mut tx, id, &BridgeScrape::Unchecked)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        assert_eq!(
            read_bridges(&pool, id).await,
            Some(prior),
            "Unchecked must leave stored bridges unchanged"
        );
    }
}
