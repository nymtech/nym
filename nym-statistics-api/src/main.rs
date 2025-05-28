use clap::Parser;
use nym_task::signal::wait_for_signal;

mod cli;
mod http;
mod logging;
mod storage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::setup_tracing_logger()?;

    let args = cli::Cli::parse();

    let connection_url = args.database_url.clone();
    tracing::debug!("Using config:\n{:#?}", args);

    let storage = storage::StatisticsStorage::init(
        connection_url,
        args.username,
        args.password,
        args.pg_port,
    )
    .await?;

    let shutdown_handles = http::server::start_http_api(storage, args.http_port)
        .await
        .expect("Failed to start server");

    tracing::info!("Started HTTP server on port {}", args.http_port);

    wait_for_signal().await;

    if let Err(err) = shutdown_handles.shutdown().await {
        tracing::error!("{err}");
    };

    Ok(())
}
