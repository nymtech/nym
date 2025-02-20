use clap::Parser;
use nym_crypto::asymmetric::ed25519::PublicKey;
use nym_task::signal::wait_for_signal;

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

    // Start the monitor
    let args_clone = args.clone();

    tokio::spawn(async move {
        monitor::spawn_in_background(
            db_pool,
            args_clone.nym_api_client_timeout,
            args_clone.nyxd_addr,
            args_clone.monitor_refresh_interval,
            args_clone.ipinfo_api_token,
            args_clone.geodata_ttl,
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
