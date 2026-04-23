use crate::db::{DbPool, models::GatewaySessionsRecord, queries};
use nym_network_defaults::NymNetworkDetails;
use nym_node_requests::api::{client::NymNodeApiClientExt, v1::metrics::models::SessionStats};
use nym_validator_client::{
    client::{NodeId, NymNodeDetails},
    models::{DescribedNodeTypeV1, NymNodeDescriptionV1},
};
use time::OffsetDateTime;

use nym_bin_common::bin_info;
use nym_node_requests::try_get_valid_nym_node_api_client;
use nym_statistics_common::types::SessionType;
use nym_validator_client::client::NymApiClientExt;
use std::collections::HashMap;
use tokio::time::Duration;
use tracing::instrument;

const FAILURE_RETRY_DELAY: Duration = Duration::from_secs(60);
const REFRESH_INTERVAL: Duration = Duration::from_secs(60 * 60 * 6);
const STALE_DURATION: Duration = Duration::from_secs(86400 * 365); //one year

#[instrument(level = "info", name = "metrics_scraper", skip_all)]
pub(crate) async fn run_in_background(db_pool: DbPool, nym_api_client_timeout: Duration) {
    let network_defaults = nym_network_defaults::NymNetworkDetails::new_from_env();

    loop {
        tracing::info!("Refreshing node self-described metrics...");

        if let Err(e) = run(&db_pool, &network_defaults, nym_api_client_timeout).await {
            tracing::error!(
                "Metrics collection failed: {e}, retrying in {}s...",
                FAILURE_RETRY_DELAY.as_secs()
            );

            tokio::time::sleep(FAILURE_RETRY_DELAY).await;
        } else {
            tracing::info!(
                "Metrics successfully collected, sleeping for {}s...",
                REFRESH_INTERVAL.as_secs()
            );
            tokio::time::sleep(REFRESH_INTERVAL).await;
        }
    }
}

async fn run(
    pool: &DbPool,
    network_details: &NymNetworkDetails,
    nym_api_client_timeout: Duration,
) -> anyhow::Result<()> {
    let default_api_url = network_details
        .endpoints
        .first()
        .expect("rust sdk mainnet default incorrectly configured")
        .api_url()
        .clone()
        .expect("rust sdk mainnet default missing api_url");

    let nym_api = nym_http_api_client::ClientBuilder::new_with_urls(vec![default_api_url.into()])?
        .no_hickory_dns()
        .with_timeout(nym_api_client_timeout)
        .build()?;

    //SW TBC what nodes exactly need to be scraped, the skimmed node endpoint seems to return more nodes
    let bonded_nodes = nym_api.get_all_bonded_nym_nodes().await?;

    // for now allow usage of old endpoint for we don't need LP related data
    // and the new endpoint might not be immediately deployed
    #[allow(deprecated)]
    let all_nodes = nym_api.get_all_described_nodes().await?; //legacy node that did not upgrade the contract bond yet
    tracing::debug!("Fetched {} total nodes", all_nodes.len());

    let mut nodes_to_scrape: HashMap<NodeId, MetricsScrapingData> = bonded_nodes
        .into_iter()
        .map(|n| (n.node_id(), n.into()))
        .collect();

    all_nodes
        .into_iter()
        .filter(|n| n.contract_node_type != DescribedNodeTypeV1::LegacyMixnode)
        .for_each(|n| {
            nodes_to_scrape.entry(n.node_id).or_insert_with(|| n.into());
        });
    tracing::debug!("Will try to scrape {} nodes", nodes_to_scrape.len());

    let mut session_records = Vec::new();
    for n in nodes_to_scrape.into_values() {
        if let Some(stat) = n.try_scrape_metrics().await {
            session_records.push(prepare_session_data(stat, &n));
        }
    }

    queries::insert_session_records(pool, session_records)
        .await
        .map(|_| {
            tracing::debug!("Session info written to DB!");
        })?;
    let cut_off_date = (OffsetDateTime::now_utc() - STALE_DURATION).date();
    queries::delete_old_records(pool, cut_off_date)
        .await
        .map(|_| {
            tracing::debug!("Cleared old data before {}", cut_off_date);
        })?;

    Ok(())
}

#[derive(Debug)]
struct MetricsScrapingData {
    host: String,
    node_id: NodeId,
    id_key: String,
    port: Option<u16>,
}

impl MetricsScrapingData {
    pub fn new(
        host: impl Into<String>,
        node_id: NodeId,
        id_key: String,
        port: Option<u16>,
    ) -> Self {
        MetricsScrapingData {
            host: host.into(),
            node_id,
            id_key,
            port,
        }
    }

    #[instrument(level = "info", name = "metrics_scraper", skip_all)]
    async fn try_scrape_metrics(&self) -> Option<SessionStats> {
        let client_fut =
            try_get_valid_nym_node_api_client(&self.host, self.node_id, self.port, bin_info!());

        match client_fut.await {
            Ok(client) => {
                match client.get_sessions_metrics().await {
                    Ok(session_stats) => {
                        if session_stats.update_time != OffsetDateTime::UNIX_EPOCH {
                            Some(session_stats)
                        } else {
                            //means no data
                            None
                        }
                    }
                    Err(e) => {
                        tracing::warn!("{e}");
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!("{e}");
                None
            }
        }
    }
}

impl From<NymNodeDetails> for MetricsScrapingData {
    fn from(value: NymNodeDetails) -> Self {
        MetricsScrapingData::new(
            value.bond_information.node.host.clone(),
            value.node_id(),
            value.bond_information.node.identity_key,
            value.bond_information.node.custom_http_port,
        )
    }
}

impl From<NymNodeDescriptionV1> for MetricsScrapingData {
    fn from(value: NymNodeDescriptionV1) -> Self {
        MetricsScrapingData::new(
            value.description.host_information.ip_address[0].to_string(),
            value.node_id,
            value.ed25519_identity_key().to_base58_string(),
            None,
        )
    }
}

fn prepare_session_data(
    stat: SessionStats,
    node_data: &MetricsScrapingData,
) -> GatewaySessionsRecord {
    let users_hashes = if !stat.unique_active_users_hashes.is_empty() {
        Some(serde_json::to_string(&stat.unique_active_users_hashes).unwrap())
    } else {
        None
    };
    let vpn_durations = stat
        .sessions
        .iter()
        .filter(|s| SessionType::from_string(&s.typ) == SessionType::Vpn)
        .map(|s| s.duration_ms)
        .collect::<Vec<_>>();

    let mixnet_durations = stat
        .sessions
        .iter()
        .filter(|s| SessionType::from_string(&s.typ) == SessionType::Mixnet)
        .map(|s| s.duration_ms)
        .collect::<Vec<_>>();

    let unknown_durations = stat
        .sessions
        .iter()
        .filter(|s| SessionType::from_string(&s.typ) == SessionType::Unknown)
        .map(|s| s.duration_ms)
        .collect::<Vec<_>>();

    let vpn_sessions = if !vpn_durations.is_empty() {
        Some(serde_json::to_string(&vpn_durations).unwrap())
    } else {
        None
    };
    let mixnet_sessions = if !mixnet_durations.is_empty() {
        Some(serde_json::to_string(&mixnet_durations).unwrap())
    } else {
        None
    };
    let unknown_sessions = if !unknown_durations.is_empty() {
        Some(serde_json::to_string(&unknown_durations).unwrap())
    } else {
        None
    };

    GatewaySessionsRecord {
        gateway_identity_key: node_data.id_key.clone(),
        node_id: node_data.node_id as i64,
        day: stat.update_time.date(),
        unique_active_clients: stat.unique_active_users as i64,
        session_started: stat.sessions_started as i64,
        users_hashes,
        vpn_sessions,
        mixnet_sessions,
        unknown_sessions,
    }
}
