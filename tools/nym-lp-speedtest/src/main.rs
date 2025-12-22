//! LP+KCP Mixnet Speedtest Client
//!
//! A client that registers with the Nym mixnet using LP transport,
//! sends traffic through Sphinx routing with KCP framing,
//! and measures network performance.

mod client;
mod speedtest;
mod topology;

use anyhow::Result;
use clap::Parser;
use tracing::info;
use url::Url;

#[derive(Parser, Debug)]
#[command(name = "nym-lp-speedtest")]
#[command(about = "LP+KCP mixnet speedtest client")]
struct Cli {
    /// Nym API URL for topology discovery
    #[arg(long, default_value = "https://validator.nymtech.net/api")]
    nym_api: Url,

    /// Specific gateway identity to test (random if not specified)
    #[arg(long)]
    gateway: Option<String>,

    /// Number of ping iterations
    #[arg(long, default_value = "10")]
    ping_count: u32,

    /// Timeout in seconds
    #[arg(long, default_value = "30")]
    timeout: u64,

    /// Output format
    #[arg(long, default_value = "json")]
    format: OutputFormat,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum OutputFormat {
    Json,
    Pretty,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    info!("Starting LP+KCP speedtest");
    info!("Nym API: {}", cli.nym_api);

    // TODO: Phase 1 - Fetch topology
    // let topology = topology::fetch_topology(&cli.nym_api).await?;

    // TODO: Phase 2 - Create client and connect
    // let mut client = client::LpSphinxClient::new(...).await?;

    // TODO: Phase 3 - Run speedtest
    // let results = speedtest::run(&mut client, cli.ping_count).await?;

    // TODO: Output results
    // match cli.format {
    //     OutputFormat::Json => println!("{}", serde_json::to_string(&results)?),
    //     OutputFormat::Pretty => println!("{}", serde_json::to_string_pretty(&results)?),
    // }

    info!("Speedtest complete");
    Ok(())
}
