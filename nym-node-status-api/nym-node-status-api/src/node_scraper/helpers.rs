use crate::node_scraper::models::{BridgeInformation, BridgeScrape};
use crate::{
    db::{
        DbPool,
        models::{InsertNodeScraperRecords, NodeStats, ScrapeNodeKind, ScraperNodeInfo},
        queries::insert_scraped_node_description,
    },
    utils::{generate_node_name, now_utc},
};
use ammonia::Builder;
use anyhow::{Result, anyhow};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::Transaction;
use std::time::Duration;
use time::UtcDateTime;
use tracing::{error, trace, warn};

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

pub async fn scrape_and_store_description_by_node_id(pool: &DbPool, node_id: i64) -> Result<()> {
    let nodes = crate::db::queries::get_nodes_for_scraping(pool).await?;
    match nodes.iter().find(|n| *n.node_kind.node_id() == node_id) {
        Some(node) => Ok(scrape_and_store_description(pool, node.clone()).await?),
        None => {
            error!("Could not find node with id {node_id}");
            Err(anyhow!("Could not find node with id {node_id}"))
        }
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

    let sanitized_description = sanitize_description(description.clone(), *node.node_id());

    trace!("tried_url_list = {tried_url_list:?}");
    trace!("ndoe_id = {}", node.node_id());
    trace!("description = {:?}", description);
    trace!("sanitized_description = {:?}", sanitized_description);

    insert_scraped_node_description(pool, &node.node_kind, &sanitized_description).await?;

    Ok(())
}

pub async fn scrape_node(node: &ScraperNodeInfo) -> Result<InsertNodeScraperRecords> {
    let client = build_client()?;
    let urls = node.contact_addresses();

    let mut stats = None;
    let mut error = None;
    let mut tried_url_list = Vec::new();
    let mut bridges = BridgeScrape::Unchecked;

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

        // Same contact URL that answered /stats: ask for published bridges
        // (entry/exit only — mixnodes never store `gateways.bridges`).
        if stats.is_some() {
            if matches!(node.node_kind, ScrapeNodeKind::EntryExitNymNode { .. }) {
                bridges = scrape_bridges(
                    &client,
                    url.trim_end_matches('/'),
                    *node.node_kind.node_id(),
                )
                .await;
            }
            break;
        }
    }

    let stats = stats.ok_or_else(|| {
        let err_msg = error.map_or_else(|| "Unknown error".to_string(), |e| e.to_string());
        anyhow::anyhow!("Failed to fetch packet stats from any URL: {}", err_msg)
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

/// Fetch client-params from the same base URL that already answered `/stats`.
///
/// - 200 + parse → `Found` (overwrite stored bridges)
/// - 404 → `NotPublished` (clear stored bridges). Covers both “operator stopped
///   publishing” and “route never mounted” (no `bridge_client_params` config).
/// - anything else → `Unchecked` (leave stored bridges alone)
async fn scrape_bridges(client: &reqwest::Client, base: &str, node_id: i64) -> BridgeScrape {
    let url = format!("{base}{BRIDGES_URL}");
    let response = match client.get(&url).send().await {
        Ok(response) => response,
        Err(err) => {
            warn!("bridge client-params unreachable for node {node_id} at {url}: {err}");
            return BridgeScrape::Unchecked;
        }
    };

    let status = response.status();
    if status == StatusCode::NOT_FOUND {
        trace!("node {node_id} does not publish bridges at {url}");
        return BridgeScrape::NotPublished;
    }
    if !status.is_success() {
        warn!("bridge client-params for node {node_id} at {url}: HTTP {status}");
        return BridgeScrape::Unchecked;
    }

    match response.json::<BridgeInformation>().await {
        Ok(info) => {
            trace!("got bridge info from node id {node_id}: {info:?}");
            BridgeScrape::Found(info)
        }
        Err(err) => {
            error!("Failed to deserialize bridge information for node {node_id}: {err}");
            BridgeScrape::Unchecked
        }
    }
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
    use axum::Router;
    use axum::http::StatusCode as AxumStatus;
    use axum::response::{IntoResponse, Response};
    use axum::routing::get;
    use serde_json::json;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio::net::TcpListener;

    const VALID_CLIENT_PARAMS: &str = r#"{"version":"0","transports":[{"transport_type":"quic_plain","args":{"addresses":["1.2.3.4:4443"],"host":"example.com","id_pubkey":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="}}],"transports_v2":[]}"#;

    async fn serve_bridges(
        on_request: Arc<dyn Fn() -> Response + Send + Sync>,
    ) -> (reqwest::Client, String) {
        let app = Router::new().route(
            BRIDGES_URL,
            get(move || {
                let on_request = Arc::clone(&on_request);
                async move { on_request() }
            }),
        );
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .expect("bind test listener");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve test app");
        });
        (reqwest::Client::new(), format!("http://{addr}"))
    }

    #[test]
    fn test_get_packet_value() {
        let json = json!({ "packets": 100, "dropped": 50.5 });
        assert_eq!(get_packet_value(&json, "packets"), Some(100));
        assert_eq!(get_packet_value(&json, "dropped"), Some(50));
        assert_eq!(get_packet_value(&json, "non_existent"), None);
    }

    #[tokio::test]
    async fn scrape_bridges_404_is_not_published() {
        let (client, base) =
            serve_bridges(Arc::new(|| AxumStatus::NOT_FOUND.into_response())).await;
        assert!(matches!(
            scrape_bridges(&client, &base, 1).await,
            BridgeScrape::NotPublished
        ));
    }

    #[tokio::test]
    async fn scrape_bridges_valid_body_is_found() {
        let (client, base) = serve_bridges(Arc::new(|| VALID_CLIENT_PARAMS.into_response())).await;
        match scrape_bridges(&client, &base, 1).await {
            BridgeScrape::Found(info) => assert_eq!(info.version, "0"),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn scrape_bridges_empty_transports_is_still_found() {
        // 200 with a valid empty descriptor overwrites storage (unlike 404 clear).
        // Callers that treat `bridges: Some` as “try bridges” should be aware.
        let body = r#"{"version":"0","transports":[],"transports_v2":[]}"#;
        let (client, base) = serve_bridges(Arc::new(|| body.into_response())).await;
        match scrape_bridges(&client, &base, 1).await {
            BridgeScrape::Found(info) => {
                assert!(info.transports.is_empty());
                assert!(info.transports_v2.is_empty());
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn scrape_bridges_garbage_body_is_unchecked() {
        let (client, base) = serve_bridges(Arc::new(|| "not-json".into_response())).await;
        assert!(matches!(
            scrape_bridges(&client, &base, 1).await,
            BridgeScrape::Unchecked
        ));
    }

    #[tokio::test]
    async fn scrape_bridges_5xx_is_unchecked() {
        let (client, base) = serve_bridges(Arc::new(|| {
            AxumStatus::INTERNAL_SERVER_ERROR.into_response()
        }))
        .await;
        assert!(matches!(
            scrape_bridges(&client, &base, 1).await,
            BridgeScrape::Unchecked
        ));
    }

    #[tokio::test]
    async fn scrape_bridges_unreachable_is_unchecked() {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(200))
            .build()
            .unwrap();
        // Nothing listening — connect failure must not clear stored bridges.
        assert!(matches!(
            scrape_bridges(&client, "http://127.0.0.1:1", 1).await,
            BridgeScrape::Unchecked
        ));
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
