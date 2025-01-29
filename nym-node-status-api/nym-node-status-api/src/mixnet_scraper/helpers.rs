use crate::db::{
    models::ScraperNodeInfo,
    queries::{insert_node_packet_stats, insert_scraped_node_description},
};
use ammonia::Builder;
use anyhow::Result;
use chrono::{DateTime, Datelike, Utc};
use reqwest;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeStats {
    pub packets_received: i64,
    pub packets_sent: i64,
    pub packets_dropped: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeDescriptionResponse {
    pub moniker: Option<String>,
    pub website: Option<String>,
    pub security_contact: Option<String>,
    pub details: Option<String>,
}

// Eventhough they are `/api/v1/*`, newer nodes have a redirect to the correct endpoints
const DESCRIPTION_URL: &str = "/description";
const PACKET_STATS_URL: &str = "/stats";

// We need this as some of the mixnodes respond with float values for the packet statistics (?????)
pub fn get_packet_value(response: &serde_json::Value, key: &str) -> Option<i64> {
    response
        .get(key)
        .and_then(|value| value.as_i64().or_else(|| value.as_f64().map(|f| f as i64)))
}

pub fn parse_mixnet_stats(response: serde_json::Value) -> Option<NodeStats> {
    // Try to parse the response return by old (deprecated) mixnodes
    if let Some(packets_dropped) =
        get_packet_value(&response, "packets_explicitly_dropped_since_startup")
    {
        return Some(NodeStats {
            packets_dropped,
            packets_sent: get_packet_value(&response, "packets_sent_since_startup")
                .unwrap_or_default(),

            packets_received: get_packet_value(&response, "packets_received_since_startup")
                .unwrap_or_default(),
        });
    }

    // Try to parse the response returned by nym-nodes
    if let Some(packets_dropped) = get_packet_value(&response, "dropped_since_startup") {
        return Some(NodeStats {
            packets_dropped,
            packets_sent: get_packet_value(&response, "sent_since_startup").unwrap_or_default(),

            packets_received: get_packet_value(&response, "received_since_startup")
                .unwrap_or_default(),
        });
    }

    // If neither format matches, return None
    None
}

pub fn calculate_packet_difference(current: i64, previous: i64) -> i64 {
    if current >= previous {
        current - previous
    } else {
        current // Node likely restarted, use current value
    }
}

pub fn build_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        // Since we have a mix of TLS and non-TLS nodes, we need to accept invalid certs
        // when accessing IP:port
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))
}

pub fn sanitize_description(description: NodeDescriptionResponse) -> NodeDescriptionResponse {
    let mut sanitizer = Builder::new();
    sanitizer
        .tags(std::collections::HashSet::new())
        .generic_attributes(std::collections::HashSet::new())
        .url_schemes(std::collections::HashSet::new());

    let sanitize_field = |opt: Option<String>| -> Option<String> {
        Some(
            opt.filter(|s| !s.trim().is_empty())
                .map_or_else(|| "N/A".to_string(), |s| sanitizer.clean(&s).to_string()),
        )
    };

    NodeDescriptionResponse {
        moniker: sanitize_field(description.moniker),
        website: sanitize_field(description.website),
        security_contact: sanitize_field(description.security_contact),
        details: sanitize_field(description.details),
    }
}

pub async fn scrape_and_store_description(pool: &SqlitePool, node: &ScraperNodeInfo) -> Result<()> {
    let client = build_client()?;
    let urls = node.contact_addresses();

    let mut description = None;
    let mut error = None;

    for mut url in urls {
        url = format!("{}{}", url.trim_end_matches('/'), DESCRIPTION_URL);

        match client.get(&url).send().await {
            Ok(response) => {
                if let Ok(desc) = response.json::<NodeDescriptionResponse>().await {
                    description = Some(desc);
                    break;
                }
            }
            Err(e) => error = Some(e),
        }
    }

    let description = description.ok_or_else(|| {
        let err_msg = error.map_or_else(|| "Unknown error".to_string(), |e| e.to_string());
        anyhow::anyhow!("Failed to fetch description from any URL: {}", err_msg)
    })?;

    let sanitized_description = sanitize_description(description);
    insert_scraped_node_description(pool, &node.node_kind, node.node_id, &sanitized_description)
        .await?;

    Ok(())
}

pub async fn scrape_and_store_packet_stats(
    pool: &SqlitePool,
    node: &ScraperNodeInfo,
) -> Result<()> {
    let client = build_client()?;
    let urls = node.contact_addresses();

    let mut stats = None;
    let mut error = None;

    for mut url in urls {
        url = format!("{}{}", url.trim_end_matches('/'), PACKET_STATS_URL);

        match client.get(&url).send().await {
            Ok(response) => {
                if let Some(node_stats) = parse_mixnet_stats(response.json().await?) {
                    stats = Some(node_stats);
                    break;
                }
            }
            Err(e) => error = Some(e),
        }
    }

    let stats = stats.ok_or_else(|| {
        let err_msg = error.map_or_else(|| "Unknown error".to_string(), |e| e.to_string());
        anyhow::anyhow!("Failed to fetch stats from any URL: {}", err_msg)
    })?;

    insert_node_packet_stats(pool, node.node_id, &node.node_kind, stats).await?;

    // TODO dz uncomment
    // Update daily stats
    // update_daily_stats(pool, node.node_id, timestamp, &stats).await?;

    Ok(())
}

pub async fn update_daily_stats(
    pool: &SqlitePool,
    node_id: i64,
    timestamp: DateTime<Utc>,
    current_stats: &NodeStats,
) -> Result<()> {
    let mut conn = pool.acquire().await?;

    let date_utc = format!(
        "{:04}-{:02}-{:02}",
        timestamp.year(),
        timestamp.month(),
        timestamp.day()
    );

    let total_stake = sqlx::query_scalar!(
        r#"
        SELECT
            total_stake
        FROM mixnodes
        WHERE mix_id = ?
        "#,
        node_id
    )
    .fetch_one(&mut *conn)
    .await?;

    // Get previous stats
    let previous_stats = sqlx::query!(
        r#"
        SELECT packets_received, packets_sent, packets_dropped
        FROM mixnode_packet_stats_raw
        WHERE mix_id = ?
        ORDER BY timestamp_utc DESC
        LIMIT 1 OFFSET 1
        "#,
        node_id
    )
    .fetch_optional(&mut *conn)
    .await?;

    let (diff_received, diff_sent, diff_dropped) = if let Some(prev) = previous_stats {
        (
            calculate_packet_difference(
                current_stats.packets_received,
                prev.packets_received.unwrap_or(0),
            ),
            calculate_packet_difference(current_stats.packets_sent, prev.packets_sent.unwrap_or(0)),
            calculate_packet_difference(
                current_stats.packets_dropped,
                prev.packets_dropped.unwrap_or(0),
            ),
        )
    } else {
        (0, 0, 0) // No previous stats available
    };

    sqlx::query!(
        r#"
        INSERT INTO mixnode_daily_stats (
            mix_id, date_utc, total_stake, packets_received, packets_sent, packets_dropped
        ) VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(mix_id, date_utc) DO UPDATE SET
            total_stake = excluded.total_stake,
            packets_received = mixnode_daily_stats.packets_received + excluded.packets_received,
            packets_sent = mixnode_daily_stats.packets_sent + excluded.packets_sent,
            packets_dropped = mixnode_daily_stats.packets_dropped + excluded.packets_dropped
        "#,
        node_id,
        date_utc,
        total_stake,
        diff_received,
        diff_sent,
        diff_dropped,
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}
