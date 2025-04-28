use crate::monitor::DelegationsCache;
use clap::Parser;
use nym_crypto::asymmetric::ed25519::PublicKey;
use nym_task::signal::wait_for_signal;
use nym_validator_client::nyxd::NyxdClient;
use std::sync::Arc;

mod cli;
mod db;
mod http;
mod logging;
mod mixnet_scraper;
mod monitor;
mod node_scraper;
mod testruns;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::setup_tracing_logger()?;

    let args = cli::Cli::parse();

    let agent_key_list = args
        .agent_key_list
        .iter()
        .map(|value| PublicKey::from_base58_string(value.trim()).map_err(anyhow::Error::from))
        .collect::<anyhow::Result<Vec<_>>>()?;
    tracing::info!("Registered {} agent keys", agent_key_list.len());

    let connection_url = args.database_url.clone();
    tracing::debug!("Using config:\n{:#?}", args);

    let storage = db::Storage::init(connection_url).await?;
    let db_pool = storage.pool_owned();

    // Start the node scraper
    let scraper = mixnet_scraper::Scraper::new(storage.pool_owned());
    tokio::spawn(async move {
        scraper.start().await;
    });

    // node geocache is shared between node monitor and HTTP server
    let geocache = moka::future::Cache::builder()
        .time_to_live(args.geodata_ttl)
        .build();
    let delegations_cache = DelegationsCache::new();

    // Start the monitor
    let args_clone = args.clone();
    let geocache_clone = geocache.clone();
    let delegations_cache_clone = Arc::clone(&delegations_cache);
    let config = nym_validator_client::nyxd::Config::try_from_nym_network_details(
        &nym_network_defaults::NymNetworkDetails::new_from_env(),
    )?;
    let nyxd_client = NyxdClient::connect(config, args.nyxd_addr.as_str())
        .map_err(|err| anyhow::anyhow!("Couldn't connect: {}", err))?;

    tokio::spawn(async move {
        monitor::spawn_in_background(
            db_pool,
            args_clone.nym_api_client_timeout,
            nyxd_client,
            args_clone.monitor_refresh_interval,
            args_clone.ipinfo_api_token,
            geocache_clone,
            delegations_cache_clone,
        )
        .await;
        tracing::info!("Started monitor task");
    });

    testruns::spawn(storage.pool_owned(), args.testruns_refresh_interval).await;

    let db_pool_scraper = storage.pool_owned();
    tokio::spawn(async move {
        node_scraper::spawn_in_background(db_pool_scraper, args_clone.nym_api_client_timeout).await;
        tracing::info!("Started metrics scraper task");
    });

    let shutdown_handles = http::server::start_http_api(
        storage.pool_owned(),
        args.http_port,
        args.nym_http_cache_ttl,
        agent_key_list.to_owned(),
        args.max_agent_count,
        geocache,
        delegations_cache,
    )
    .await
    .expect("Failed to start server");

    tracing::info!("Started HTTP server on port {}", args.http_port);

    wait_for_signal().await;

    if let Err(err) = shutdown_handles.shutdown().await {
        tracing::error!("{err}");
    };

    Ok(())
}
