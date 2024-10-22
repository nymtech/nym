use clap::Parser;
use nym_network_defaults::setup_env;
use nym_task::signal::wait_for_signal;

mod background_task;
mod db;
mod http;
mod logging;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(long, default_value_t = 8000, env = "NYM_DATA_OBSERVATORY_HTTP_PORT")]
    http_port: u16,

    /// Path to the environment variables file. If you don't provide one, variables for the mainnet will be used.
    #[arg(short, long, default_value = None, env = "NYM_DATA_OBSERVATORY_ENV_FILE")]
    env_file: Option<String>,

    /// DB connection url
    #[arg(short, long, default_value = None, env = "NYM_DATA_OBSERVATORY_CONNECTION_URL")]
    connection_url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::setup_tracing_logger();

    let args = Args::parse();

    setup_env(args.env_file); // Defaults to mainnet if empty

    let storage = db::Storage::init(args.connection_url).await?;
    let db_pool = storage.pool_owned().await;
    tokio::spawn(async move {
        background_task::spawn_in_background(db_pool).await;
        tracing::info!("Started task");
    });

    let shutdown_handles = http::server::start_http_api(storage.pool_owned().await, args.http_port)
        .await
        .expect("Failed to start server");
    tracing::info!("Started HTTP server on port {}", args.http_port);

    wait_for_signal().await;

    if let Err(err) = shutdown_handles.shutdown().await {
        tracing::error!("{err}");
    };

    Ok(())
}
