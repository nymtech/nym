use clap::Parser;
use nym_task::signal::wait_for_signal;

mod cli;
mod db;
mod http;
mod logging;
mod monitor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::setup_tracing_logger();

    let args = cli::Cli::parse();

    let connection_url = args.connection_url.clone();
    tracing::debug!("Using config:\n{:#?}", args);

    let storage = db::Storage::init(connection_url).await?;
    let db_pool = storage.pool_owned().await;
    let args_clone = args.clone();
    tokio::spawn(async move {
        monitor::spawn_in_background(db_pool, args_clone).await;
    });
    tracing::info!("Started monitor task");

    let shutdown_handles = http::server::start_http_api(
        storage.pool_owned().await,
        args.http_port,
        args.nym_http_cache_ttl,
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
