//! LP+KCP Mixnet Speedtest Client
//!
//! A client that registers with the Nym mixnet using LP transport,
//! sends traffic through Sphinx routing with KCP framing,
//! and measures network performance.

mod client;
mod speedtest;
mod topology;

use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use rand::thread_rng;
use tracing::{error, info};
use url::Url;

use client::SpeedtestClient;
use topology::SpeedtestTopology;

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

    // Fetch topology
    info!("Fetching network topology...");
    let topology = SpeedtestTopology::fetch(&cli.nym_api)
        .await
        .context("failed to fetch topology")?;

    info!("Topology loaded: {} entry gateways", topology.gateway_count());

    // Select gateway
    let mut rng = thread_rng();
    let gateway = match &cli.gateway {
        Some(identity) => topology.gateway_by_identity(identity)?.clone(),
        None => topology.random_gateway(&mut rng)?.clone(),
    };

    info!("Selected gateway: {}", gateway.identity);
    info!("  LP address: {}", gateway.lp_address);
    info!("  Mix host: {}", gateway.mix_host);

    // Test LP handshake
    let topology = Arc::new(topology);
    let mut client = SpeedtestClient::new(gateway, topology);

    match client.test_lp_handshake().await {
        Ok(duration) => info!("LP handshake successful in {:?}", duration),
        Err(e) => {
            error!("LP handshake failed: {}", e);
            return Err(e);
        }
    }

    // Test data path through mixnet
    info!("Testing data path through mixnet...");
    let test_payload = b"Hello from nym-lp-speedtest!";

    // Test one-way send (no SURBs)
    match client.send_data(test_payload).await {
        Ok(()) => info!("One-way data send successful"),
        Err(e) => error!("One-way data send failed: {}", e),
    }

    // Test send with SURBs (for bidirectional capability)
    match client.send_data_with_surbs(test_payload, 3).await {
        Ok(keys) => {
            info!(
                "Data with {} SURBs sent successfully (reply keys stored)",
                keys.len()
            );
        }
        Err(e) => error!("Data with SURBs send failed: {}", e),
    }

    info!("Speedtest complete");
    Ok(())
}
