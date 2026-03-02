use crate::cli::Commands;
use crate::monitor::DelegationsCache;
use crate::node_scraper::helpers::scrape_and_store_description_by_node_id;
use crate::ticketbook_manager::TicketbookManager;
use crate::ticketbook_manager::state::TicketbookManagerState;
use clap::Parser;
use nym_credential_proxy_lib::quorum_checker::QuorumStateChecker;
use nym_credential_proxy_lib::shared_state::nyxd_client::ChainClient;
use nym_crypto::asymmetric::ed25519::PublicKey;
use nym_network_defaults::setup_env;
use nym_task::ShutdownManager;
use nym_validator_client::nyxd::NyxdClient;
use std::{collections::HashMap, sync::Arc};

mod cli;
mod db;
mod http;
mod logging;
mod metrics_scraper;
mod monitor;
mod node_scraper;
mod testruns;
mod ticketbook_manager;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::setup_tracing_logger()?;

    let args = cli::Cli::parse();
    if let Some(env_file) = &args.config_env_file {
        setup_env(Some(env_file));
    }
    let network = nym_network_defaults::NymNetworkDetails::new_from_env();

    let mut shutdown_manager = ShutdownManager::build_new_default()?;

    let agent_key_list = args
        .agent_key_list
        .iter()
        .map(|value| PublicKey::from_base58_string(value.trim()).map_err(anyhow::Error::from))
        .collect::<anyhow::Result<Vec<_>>>()?;
    tracing::info!("Registered {} agent keys", agent_key_list.len());
    let agent_region_map = parse_agent_region_map(args.agent_region_map.as_deref())?;
    let region_centroids = parse_region_centroids(args.region_centroids.as_deref())?;
    tracing::info!(
        "Configured {} agent region mappings and {} region centroids",
        agent_region_map.len(),
        region_centroids.len()
    );

    let connection_url = args.database_url.clone();
    if std::env::var("SHOW_CONFIG").ok().is_some() {
        tracing::debug!("Using config:\n{:#?}", args);
    }

    let storage = db::Storage::init(
        connection_url,
        args.sqlx_busy_timeout_s,
        args.sqlx_max_connections,
        args.sqlx_min_connections,
    )
    .await?;
    let db_pool = storage.pool_owned();

    // node geocache is shared between node monitor and HTTP server
    let geocache = moka::future::Cache::builder()
        .time_to_live(args.geodata_ttl)
        .build();
    let delegations_cache = DelegationsCache::new();

    let client_config = nym_validator_client::nyxd::Config::try_from_nym_network_details(&network)?;
    tracing::info!("Network: {}", network.network_name);

    let nyxd_client = NyxdClient::connect(client_config.clone(), args.nyxd_addr.as_str())
        .map_err(|err| anyhow::anyhow!("Couldn't connect: {}", err))?;

    match args.command {
        Some(Commands::ScrapeNode { node_id }) => {
            if std::env::var("RUN_ONCE_INIT_NODES").ok().is_some() {
                let geocache_clone = geocache.clone();
                let delegations_cache_clone = Arc::clone(&delegations_cache);
                monitor::run_once(
                    db_pool.clone(),
                    args.nym_api_client_timeout,
                    nyxd_client,
                    args.ipinfo_api_token,
                    geocache_clone,
                    delegations_cache_clone,
                )
                .await?;
            }
            tracing::info!("Scraping node with id {node_id}...");
            scrape_and_store_description_by_node_id(&db_pool, node_id).await?;
            return Ok(());
        }
        None => {
            // default behaviour
        }
    }

    // Start the node scraper
    let scraper = node_scraper::DescriptionScraper::new(storage.pool_owned());
    shutdown_manager.spawn_with_shutdown(async move {
        scraper.start().await;
    });
    let scraper = node_scraper::NodeScraper::new(
        storage.pool_owned(),
        args.packet_stats_max_concurrent_tasks,
    );
    shutdown_manager.spawn_with_shutdown(async move {
        scraper.start().await;
    });

    // Start the monitor
    let geocache_clone = geocache.clone();
    let delegations_cache_clone = Arc::clone(&delegations_cache);

    shutdown_manager.spawn_with_shutdown(async move {
        monitor::run_in_background(
            db_pool,
            args.nym_api_client_timeout,
            nyxd_client,
            args.monitor_refresh_interval,
            args.ipinfo_api_token,
            geocache_clone,
            delegations_cache_clone,
        )
        .await;
        tracing::info!("Started monitor task");
    });

    let pool = storage.pool_owned();
    shutdown_manager.spawn_with_shutdown(async move {
        testruns::start(pool, args.testruns_refresh_interval).await
    });

    let db_pool_scraper = storage.pool_owned();
    shutdown_manager.spawn_with_shutdown(async move {
        metrics_scraper::run_in_background(db_pool_scraper, args.nym_api_client_timeout).await;
        tracing::info!("Started metrics scraper task");
    });

    // >>> TICKETBOOK TASKS SETUP START
    let config = args.ticketbook.to_manager_config();

    // client for sending chain transactions
    let chain_client = ChainClient::new_with_config(
        client_config,
        args.nyxd_addr.as_str(),
        args.ticketbook.mnemonic,
    )?;

    // background task for checking for signing quorum
    let cancellation_token = shutdown_manager.clone_shutdown_token().inner().clone();
    let quorum_state_checker = QuorumStateChecker::new(
        chain_client.clone(),
        args.ticketbook.quorum_check_interval,
        cancellation_token,
    )
    .await?;
    let quorum_state = quorum_state_checker.quorum_state_ref();

    let ticketbook_manager_state = TicketbookManagerState::new(
        config.buffered_ticket_types.clone(),
        storage.clone(),
        quorum_state,
        chain_client,
    );
    // ensure initial caches are built up
    ticketbook_manager_state.build_initial_cache().await?;

    shutdown_manager.spawn(async move {
        quorum_state_checker.run_forever().await;
    });

    let shutdown_token = shutdown_manager.clone_shutdown_token();
    let ticketbook_manager = TicketbookManager::new(
        config,
        ticketbook_manager_state.clone(),
        args.ticketbook.ecash_client_identifier_bs58.0,
        shutdown_token,
    );
    shutdown_manager.spawn(async move {
        ticketbook_manager.run().await;
    });

    // >>> TICKETBOOK TASKS SETUP END

    let shutdown_tracker = shutdown_manager.shutdown_tracker();
    http::server::start_http_api(
        storage.pool_owned(),
        args.http_port,
        args.nym_http_cache_ttl,
        agent_key_list.to_owned(),
        agent_region_map,
        region_centroids,
        args.max_agent_count,
        args.agent_request_freshness,
        geocache,
        delegations_cache,
        ticketbook_manager_state,
        shutdown_tracker,
    )
    .await
    .expect("Failed to start server");

    tracing::info!("Started HTTP server on port {}", args.http_port);

    shutdown_manager.run_until_shutdown().await;

    Ok(())
}

fn parse_agent_region_map(raw: Option<&str>) -> anyhow::Result<HashMap<PublicKey, String>> {
    let mut out = HashMap::new();
    let Some(raw) = raw else {
        return Ok(out);
    };

    for entry in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (pubkey_raw, region_raw) = entry.split_once('=').ok_or_else(|| {
            anyhow::anyhow!(
                "malformed NODE_STATUS_API_AGENT_REGION_MAP entry '{entry}', expected '<pubkey>=<region>'"
            )
        })?;
        let pubkey =
            PublicKey::from_base58_string(pubkey_raw.trim()).map_err(anyhow::Error::from)?;
        let region = region_raw.trim();
        if region.is_empty() {
            anyhow::bail!("empty region in NODE_STATUS_API_AGENT_REGION_MAP entry '{entry}'");
        }
        out.insert(pubkey, region.to_string());
    }

    Ok(out)
}

fn parse_region_centroids(
    raw: Option<&str>,
) -> anyhow::Result<HashMap<String, http::state::RegionCentroid>> {
    let mut out = HashMap::new();
    let Some(raw) = raw else {
        return Ok(out);
    };

    for entry in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (region_raw, lat_lon_raw) = entry.split_once('=').ok_or_else(|| {
            anyhow::anyhow!(
                "malformed NODE_STATUS_API_REGION_CENTROIDS entry '{entry}', expected '<region>=<lat>:<lon>'"
            )
        })?;
        let region = region_raw.trim();
        let (lat_raw, lon_raw) = lat_lon_raw.split_once(':').ok_or_else(|| {
            anyhow::anyhow!(
                "malformed NODE_STATUS_API_REGION_CENTROIDS entry '{entry}', expected '<region>=<lat>:<lon>'"
            )
        })?;
        let lat = lat_raw.trim().parse::<f64>().map_err(|err| {
            anyhow::anyhow!(
                "invalid latitude '{}' in entry '{}': {err}",
                lat_raw.trim(),
                entry
            )
        })?;
        let lon = lon_raw.trim().parse::<f64>().map_err(|err| {
            anyhow::anyhow!(
                "invalid longitude '{}' in entry '{}': {err}",
                lon_raw.trim(),
                entry
            )
        })?;

        if region.is_empty() {
            anyhow::bail!("empty region in NODE_STATUS_API_REGION_CENTROIDS entry '{entry}'");
        }

        out.insert(region.to_string(), http::state::RegionCentroid { lat, lon });
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_agent_region_map() {
        let pubkey_a = nym_crypto::asymmetric::ed25519::PublicKey::from_bytes(&[1; 32])
            .expect("failed to create test public key A")
            .to_base58_string();
        let pubkey_b = nym_crypto::asymmetric::ed25519::PublicKey::from_bytes(&[2; 32])
            .expect("failed to create test public key B")
            .to_base58_string();
        let raw = format!("{pubkey_a}=eu-west,{pubkey_b}=asia-tokyo");

        let parsed = parse_agent_region_map(Some(&raw)).expect("failed to parse map");

        assert_eq!(parsed.len(), 2);
        let key_a = PublicKey::from_base58_string(&pubkey_a).expect("failed to decode key A");
        let key_b = PublicKey::from_base58_string(&pubkey_b).expect("failed to decode key B");
        assert_eq!(parsed.get(&key_a).map(String::as_str), Some("eu-west"));
        assert_eq!(parsed.get(&key_b).map(String::as_str), Some("asia-tokyo"));
    }

    #[test]
    fn malformed_agent_region_map_entry_returns_error() {
        let err = parse_agent_region_map(Some("this_is_not_valid")).expect_err("expected error");
        assert!(
            err.to_string()
                .contains("malformed NODE_STATUS_API_AGENT_REGION_MAP entry"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parses_region_centroids() {
        let raw = "eu-west=50.1109:8.6821,asia-tokyo=35.6762:139.6503";

        let parsed = parse_region_centroids(Some(raw)).expect("failed to parse centroids");

        assert_eq!(parsed.len(), 2);
        let eu = parsed.get("eu-west").expect("missing eu-west centroid");
        let asia = parsed
            .get("asia-tokyo")
            .expect("missing asia-tokyo centroid");
        assert!((eu.lat - 50.1109).abs() < 1e-9);
        assert!((eu.lon - 8.6821).abs() < 1e-9);
        assert!((asia.lat - 35.6762).abs() < 1e-9);
        assert!((asia.lon - 139.6503).abs() < 1e-9);
    }

    #[test]
    fn malformed_region_centroids_entry_returns_error() {
        let err = parse_region_centroids(Some("eu-west=50.1|8.6")).expect_err("expected error");
        assert!(
            err.to_string()
                .contains("malformed NODE_STATUS_API_REGION_CENTROIDS entry"),
            "unexpected error: {err}"
        );
    }
}
