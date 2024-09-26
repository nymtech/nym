use nym_network_defaults::setup_env;
use nym_task::signal::wait_for_signal;

use crate::http::config;

mod background_task;
mod db;
mod http;
mod logging;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::setup_tracing_logger();

    // if dotenv file is present, load its values
    // otherwise, default to mainnet
    setup_env(Some(".env"));

    let conf = config::Config::from_env();
    tracing::debug!("Using config:\n{:?}", conf);

    let storage = db::Storage::init().await?;
    let db_pool = storage.pool_owned().await;
    tokio::spawn(async move {
        background_task::spawn_in_background(db_pool).await;
        tracing::info!("Started task");
    });

    let shutdown_handles =
        http::server::start_http_api(storage.pool_owned().await, conf.http_port())
            .await
            .expect("Failed to start server");
    tracing::info!("Started HTTP server on port {}", conf.http_port());

    wait_for_signal().await;

    if let Err(err) = shutdown_handles.shutdown().await {
        tracing::error!("{err}");
    };

    Ok(())
}

// TODO dz move this to common
fn read_env_var(env_var: &str) -> anyhow::Result<String> {
    std::env::var(env_var)
        .map_err(|_| anyhow::anyhow!("You need to set {}", env_var))
        .map(|value| {
            tracing::trace!("{}={}", env_var, value);
            value
        })
}
