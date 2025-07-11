use clap::Parser;
use network_view::NetworkRefresher;
use nym_task::ShutdownManager;

mod cli;
mod http;
mod logging;
mod network_view;
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
        args.ssl_cert_path,
    )
    .await?;
    tracing::info!("Connection to database successful");

    let shutdown_manager = ShutdownManager::new("nym-statistics-api");

    let network_refresher = NetworkRefresher::initialise_new(
        args.nym_api_url,
        shutdown_manager.child_token("network-refresher"),
    )
    .await;

    let http_server =
        http::server::build_http_api(storage, network_refresher.network_view(), args.http_port)
            .await
            .expect("Failed to build http server");
    let server_shutdown = shutdown_manager.clone_token("http-api-server");

    // Starting tasks
    shutdown_manager.spawn(async move { http_server.run(server_shutdown).await });
    network_refresher.start();

    tracing::info!("Started HTTP server on port {}", args.http_port);

    shutdown_manager.close();
    shutdown_manager.run_until_shutdown().await;

    Ok(())
}
