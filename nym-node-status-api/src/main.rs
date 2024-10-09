use clap::Parser;
use nym_network_defaults::setup_env;
use nym_task::signal::wait_for_signal;

use crate::config::read_env_var;

mod cli;
mod config;
mod db;
mod http;
mod logging;
mod monitor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::setup_tracing_logger();

    let args = cli::Cli::parse();
    // if dotenv file is present, load its values
    // otherwise, default to mainnet
    setup_env(args.config_env_file.as_ref());
    tracing::debug!("{:?}", read_env_var("NETWORK_NAME"));
    tracing::debug!("{:?}", read_env_var("EXPLORER_API"));
    tracing::debug!("{:?}", read_env_var("NYM_API"));

    let conf = config::Config::from_env()?;
    tracing::debug!("Using config:\n{:#?}", conf);

    let storage = db::Storage::init().await?;
    let db_pool = storage.pool_owned().await;
    let conf_clone = conf.clone();
    tokio::spawn(async move {
        monitor::spawn_in_background(db_pool, conf_clone).await;
    });
    tracing::info!("Started monitor task");

    let shutdown_handles = http::server::start_http_api(
        storage.pool_owned().await,
        conf.http_port(),
        conf.nym_http_cache_ttl(),
    )
    .await
    .expect("Failed to start server");

    tracing::info!("Started HTTP server on port {}", conf.http_port());

    wait_for_signal().await;

    if let Err(err) = shutdown_handles.shutdown().await {
        tracing::error!("{err}");
    };

    Ok(())
}
