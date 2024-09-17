use anyhow::anyhow;
use clap::Parser;
use nym_network_defaults::setup_env;
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
    // if dotenv file is present, load its values
    // otherwise, default to mainnet
    setup_env(args.config_env_file.as_ref());
    tracing::debug!("{:?}", std::env::var("NETWORK_NAME"));
    tracing::debug!("{:?}", std::env::var("EXPLORER_API"));
    tracing::debug!("{:?}", std::env::var("NYM_API"));

    let storage = db::Storage::init().await?;
    let db_pool = storage.pool_owned().await;
    // tokio::spawn(async move {
    //     monitor::spawn_in_background(db_pool).await;
    // });
    let _ = monitor::spawn_in_background(db_pool);
    tracing::info!("Started monitor task");

    let port = std::env::var("HTTP_PORT")
        .map_err(|_| anyhow!("HTTP_PORT not set"))
        .and_then(|port| port.parse().map_err(anyhow::Error::msg))?;
    let shutdown_handles = http::server::start_http_api(storage.pool_owned().await, port)
        .await
        .expect("Failed to start server");
    // TODO dz load bind address from config
    // TODO dz log bind address
    tracing::info!("Started HTTP server on port {}", port);

    wait_for_signal().await;

    if let Err(err) = shutdown_handles.shutdown().await {
        tracing::error!("{err}");
    };

    Ok(())
}

fn read_env_var(env_var: &str) -> anyhow::Result<String> {
    std::env::var(env_var)
        .map(|value| {
            tracing::trace!("{}={}", env_var, value);
            value
        })
        .map_err(|_| anyhow!("You need to set {}", env_var))
}
