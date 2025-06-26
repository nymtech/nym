use crate::{
    db::{
        models::{NodeStats, ScraperNodeInfo},
        queries::{
            get_raw_node_stats, insert_daily_node_stats, insert_node_packet_stats,
            insert_scraped_node_description,
        },
    },
    utils::{generate_node_name, now_utc},
};
use ammonia::Builder;
use anyhow::{anyhow, Result};
use reqwest;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::time::Duration;
use time::UtcDateTime;

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

pub fn sanitize_description(
    description: NodeDescriptionResponse,
    node_id: i64,
) -> NodeDescriptionResponse {
    let mut sanitizer = Builder::new();
    sanitizer
        .tags(std::collections::HashSet::new())
        .generic_attributes(std::collections::HashSet::new())
        .url_schemes(std::collections::HashSet::new());

    const UNKNOWN: &str = "N/A";
    let sanitize_field = |opt: Option<String>| -> Option<String> {
        Some(
            opt.filter(|s| !s.trim().is_empty())
                .map_or_else(|| UNKNOWN.to_string(), |s| sanitizer.clean(&s).to_string()),
        )
    };

    let mut moniker = sanitize_field(description.moniker);
    if let Some(sanitized) = &moniker {
        if sanitized == UNKNOWN {
            moniker = Some(generate_node_name(node_id));
        }
    };

    NodeDescriptionResponse {
        moniker,
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
    let mut tried_url_list = Vec::new();

    for mut url in urls {
        url = format!("{}{}", url.trim_end_matches('/'), DESCRIPTION_URL);
        tried_url_list.push(url.clone());

        match client
            .get(&url)
            .send()
            .await
            // convert 404 and similar to error
            .and_then(|res| res.error_for_status())
        {
            Ok(response) => {
                if let Ok(desc) = response.json::<NodeDescriptionResponse>().await {
                    description = Some(desc);
                    break;
                }
            }
            Err(e) => error = Some(anyhow!("{:?} ({})", tried_url_list, e)),
        }
    }

    let description = description.ok_or_else(|| {
        let err_msg = error.map_or_else(|| "Unknown error".to_string(), |e| e.to_string());
        anyhow::anyhow!("Failed to fetch description from any URL: {}", err_msg)
    })?;

    let sanitized_description = sanitize_description(description, *node.node_id());
    insert_scraped_node_description(pool, &node.node_kind, &sanitized_description).await?;

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
    let mut tried_url_list = Vec::new();

    for mut url in urls {
        url = format!("{}{}", url.trim_end_matches('/'), PACKET_STATS_URL);
        tried_url_list.push(url.clone());

        match client.get(&url).send().await {
            Ok(response) => {
                if let Some(node_stats) = parse_mixnet_stats(response.json().await?) {
                    stats = Some(node_stats);
                    break;
                }
            }
            Err(e) => error = Some(anyhow!("{:?} ({})", tried_url_list, e)),
        }
    }

    let stats = stats.ok_or_else(|| {
        let err_msg = error.map_or_else(|| "Unknown error".to_string(), |e| e.to_string());
        anyhow::anyhow!("Failed to fetch description from any URL: {}", err_msg)
    })?;

    let timestamp = now_utc();
    let timestamp_utc = timestamp.unix_timestamp();
    insert_node_packet_stats(pool, &node.node_kind, &stats, timestamp_utc).await?;

    // TODO dz does this need to run every time?
    update_daily_stats(pool, node, timestamp, &stats).await?;

    Ok(())
}

pub async fn update_daily_stats(
    pool: &SqlitePool,
    node: &ScraperNodeInfo,
    timestamp: UtcDateTime,
    current_stats: &NodeStats,
) -> Result<()> {
    let date_utc = format!(
        "{:04}-{:02}-{:02}",
        timestamp.year(),
        timestamp.month() as u8,
        timestamp.day()
    );

    // Get previous stats
    let previous_stats = get_raw_node_stats(pool, node).await?;

    let (diff_received, diff_sent, diff_dropped) = if let Some(prev) = previous_stats {
        (
            calculate_packet_difference(current_stats.packets_received, prev.packets_received),
            calculate_packet_difference(current_stats.packets_sent, prev.packets_sent),
            calculate_packet_difference(current_stats.packets_dropped, prev.packets_dropped),
        )
    } else {
        (0, 0, 0) // No previous stats available
    };

    insert_daily_node_stats(
        pool,
        node,
        &date_utc,
        NodeStats {
            packets_received: diff_received,
            packets_sent: diff_sent,
            packets_dropped: diff_dropped,
        },
    )
    .await?;

    Ok(())
}
