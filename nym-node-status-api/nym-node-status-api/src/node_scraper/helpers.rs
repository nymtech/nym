use crate::node_scraper::models::BridgeInformation;
use crate::{
    db::{
        models::{InsertNodeScraperRecords, NodeStats, ScrapeNodeKind, ScraperNodeInfo},
        queries::insert_scraped_node_description,
        DbPool,
    },
    utils::{generate_node_name, now_utc},
};
use ammonia::Builder;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sqlx::Transaction;
use std::time::Duration;
use time::UtcDateTime;
use tracing::{error, trace};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeDescriptionResponse {
    pub moniker: Option<String>,
    pub website: Option<String>,
    pub security_contact: Option<String>,
    pub details: Option<String>,
}

// Eventhough they are `/api/v1/*`, newer nodes have a redirect to the correct endpoints
const DESCRIPTION_URL: &str = "/description";
const PACKET_STATS_URL: &str = "/stats";

const BRIDGES_URL: &str = "/api/v1/bridges/client-params";

// We need this as some of the mixnodes respond with float values for the packet statistics (?????)
pub fn get_packet_value(response: &serde_json::Value, key: &str) -> Option<i32> {
    response
        .get(key)
        .and_then(|value| value.as_i64().or_else(|| value.as_f64().map(|f| f as i64)))
        .map(|v| v as i32)
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

pub fn calculate_packet_difference(current: i32, previous: i32) -> i32 {
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
        Some(opt.filter(|s| !s.trim().is_empty()).map_or_else(
            || UNKNOWN.to_string(),
            |s| sanitizer.clean(s.trim()).to_string().trim().to_string(),
        ))
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

pub async fn scrape_and_store_description(pool: &DbPool, node: ScraperNodeInfo) -> Result<()> {
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

pub async fn scrape_node(node: &ScraperNodeInfo) -> Result<InsertNodeScraperRecords> {
    let client = build_client()?;
    let urls = node.contact_addresses();

    let mut stats = None;
    let mut error = None;
    let mut tried_url_list = Vec::new();
    let mut bridges = None;

    for url in urls {
        let url_to_try = format!("{}{}", url.trim_end_matches('/'), PACKET_STATS_URL);
        tried_url_list.push(url_to_try.clone());

        // try to get stats
        match client.get(&url_to_try).send().await {
            Ok(response) => {
                if let Some(node_stats) = parse_mixnet_stats(response.json().await?) {
                    stats = Some(node_stats);
                }
            }
            Err(e) => error = Some(anyhow!("{:?} ({})", tried_url_list, e)),
        }

        // this url worked, so scrape some other endpoints
        if stats.is_some() {
            let url_bridges = format!("{}{}", url.trim_end_matches('/'), BRIDGES_URL);
            if let Ok(response) = client
                .get(&url_bridges)
                .send()
                .await
                .and_then(|res| res.error_for_status())
            {
                let json = response.json().await?;
                bridges = serde_json::from_value::<BridgeInformation>(json)
                    .inspect_err(|err| {
                        error!("Failed to deserialize bridge information: {err}");
                    })
                    .ok();
                trace!(
                    "got bridge info from node id {}: {bridges:?}",
                    node.node_kind.node_id()
                );
            }
        }

        // if we have valid stats, we can stop trying other URLs
        if stats.is_some() {
            break;
        }
    }

    let stats = stats.ok_or_else(|| {
        let err_msg = error.map_or_else(|| "Unknown error".to_string(), |e| e.to_string());
        anyhow::anyhow!("Failed to fetch description from any URL: {}", err_msg)
    })?;

    let timestamp_utc = now_utc();
    let unix_timestamp = timestamp_utc.unix_timestamp();
    let result = InsertNodeScraperRecords {
        node_kind: node.node_kind.to_owned(),
        timestamp_utc,
        unix_timestamp,
        stats,
        bridges,
    };

    Ok(result)
}

pub async fn update_daily_stats_uncommitted(
    tx: &mut Transaction<'static, sqlx::Postgres>,
    node_kind: &ScrapeNodeKind,
    timestamp: UtcDateTime,
    current_stats: &NodeStats,
) -> Result<()> {
    use crate::db::queries::{get_raw_node_stats, insert_daily_node_stats_uncommitted};

    let date_utc = format!(
        "{:04}-{:02}-{:02}",
        timestamp.year(),
        timestamp.month() as u8,
        timestamp.day()
    );

    // Get previous stats
    let previous_stats = get_raw_node_stats(tx, node_kind).await?;

    let (diff_received, diff_sent, diff_dropped) = if let Some(prev) = previous_stats {
        (
            calculate_packet_difference(current_stats.packets_received, prev.packets_received),
            calculate_packet_difference(current_stats.packets_sent, prev.packets_sent),
            calculate_packet_difference(current_stats.packets_dropped, prev.packets_dropped),
        )
    } else {
        (0, 0, 0) // No previous stats available
    };

    insert_daily_node_stats_uncommitted(
        tx,
        node_kind,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_get_packet_value() {
        let json = json!({ "packets": 100, "dropped": 50.5 });
        assert_eq!(get_packet_value(&json, "packets"), Some(100));
        assert_eq!(get_packet_value(&json, "dropped"), Some(50));
        assert_eq!(get_packet_value(&json, "non_existent"), None);
    }

    #[test]
    fn test_parse_mixnet_stats() {
        let old_format = json!({
            "packets_explicitly_dropped_since_startup": 10,
            "packets_sent_since_startup": 100,
            "packets_received_since_startup": 200
        });
        let new_format = json!({
            "dropped_since_startup": 20,
            "sent_since_startup": 150,
            "received_since_startup": 250
        });
        let invalid_format = json!({ "foo": "bar" });

        let stats1 = parse_mixnet_stats(old_format).unwrap();
        assert_eq!(stats1.packets_dropped, 10);
        assert_eq!(stats1.packets_sent, 100);
        assert_eq!(stats1.packets_received, 200);

        let stats2 = parse_mixnet_stats(new_format).unwrap();
        assert_eq!(stats2.packets_dropped, 20);
        assert_eq!(stats2.packets_sent, 150);
        assert_eq!(stats2.packets_received, 250);

        assert!(parse_mixnet_stats(invalid_format).is_none());
    }

    #[test]
    fn test_calculate_packet_difference() {
        assert_eq!(calculate_packet_difference(100, 50), 50);
        assert_eq!(calculate_packet_difference(50, 100), 50);
        assert_eq!(calculate_packet_difference(100, 100), 0);
    }

    #[test]
    fn test_sanitize_description() {
        let desc = NodeDescriptionResponse {
            moniker: Some("  <script>alert('xss')</script> Moniker  ".to_string()),
            website: Some("https://example.com".to_string()),
            security_contact: Some("".to_string()),
            details: None,
        };

        let sanitized = sanitize_description(desc, 123);
        assert_eq!(sanitized.moniker, Some("Moniker".to_string()));
        assert_eq!(sanitized.website, Some("https://example.com".to_string()));
        assert_eq!(sanitized.security_contact, Some("N/A".to_string()));
        assert_eq!(sanitized.details, Some("N/A".to_string()));

        let desc_empty = NodeDescriptionResponse {
            moniker: None,
            website: None,
            security_contact: None,
            details: None,
        };
        let sanitized_empty = sanitize_description(desc_empty, 123);
        assert_ne!(sanitized_empty.moniker, Some("N/A".to_string())); // should generate a name
        assert_eq!(sanitized_empty.website, Some("N/A".to_string()));
        assert_eq!(sanitized_empty.security_contact, Some("N/A".to_string()));
        assert_eq!(sanitized_empty.details, Some("N/A".to_string()));
    }
}
