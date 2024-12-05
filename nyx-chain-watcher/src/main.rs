use chain_scraper::run_chain_scraper;
use clap::Parser;
use nym_network_defaults::setup_env;
use nym_task::signal::wait_for_signal;
use tokio::join;

mod chain_scraper;
mod db;
mod http;
mod logging;
mod payment_listener;
mod price_scraper;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(long, default_value_t = 8000, env = "NYX_CHAIN_WATCHER_HTTP_PORT")]
    http_port: u16,

    /// Path to the environment variables file. If you don't provide one, variables for the mainnet will be used.
    #[arg(short, long, default_value = None, env = "NYX_CHAIN_WATCHER_ENV_FILE")]
    env_file: Option<String>,

    /// SQLite database file path
    #[arg(
        long,
        default_value = "nyx_chain_watcher.sqlite",
        env = "DATABASE_URL"
    )]
    db_path: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::setup_tracing_logger();

    let args = Args::parse();
    setup_env(args.env_file); // Defaults to mainnet if empty

    let db_path = args.db_path;
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let connection_url = format!("sqlite://{}?mode=rwc", db_path);
    let storage = db::Storage::init(connection_url).await?;
    let watcher_pool = storage.pool_owned().await;

    // Spawn the chain scraper and get its storage

    // Spawn the payment listener task
    let payment_listener_handle = tokio::spawn({
        let obs_pool = watcher_pool.clone();
        let chain_storage = run_chain_scraper().await?;

        async move {
            if let Err(e) = payment_listener::run_payment_listener(obs_pool, chain_storage).await {
                tracing::error!("Payment listener error: {}", e);
            }
            Ok::<_, anyhow::Error>(())
        }
    });

    // Clone pool for each task that needs it
    //let background_pool = db_pool.clone();

    let price_scraper_handle = tokio::spawn(async move {
        price_scraper::run_price_scraper(&watcher_pool).await;
    });

    let shutdown_handles = http::server::start_http_api(storage.pool_owned().await, args.http_port)
        .await
        .expect("Failed to start server");

    tracing::info!("Started HTTP server on port {}", args.http_port);

    // Wait for the short-lived tasks to complete
    let _ = join!(price_scraper_handle, payment_listener_handle);

    // Wait for a signal to terminate the long-running task
    wait_for_signal().await;

    if let Err(err) = shutdown_handles.shutdown().await {
        tracing::error!("{err}");
    };

    Ok(())
}
