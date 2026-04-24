// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use mix_sim::{driver::MixSimDriver, node::TopologyNode, packet::SimplePacket};
use tracing::info;

#[derive(Parser)]
#[command(name = "mix-sim", about = "Nym mix network simulator")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a topology.json file with a given number of nodes
    InitTopology {
        /// Number of nodes to generate
        #[arg(short, long)]
        nodes: u8,

        /// Output file path
        #[arg(short, long, default_value = "topology.json")]
        output: String,
    },

    /// Run the mix simulation with a given topology file
    Run {
        /// Path to the topology.json file
        #[arg(short, long, default_value = "topology.json")]
        topology: String,

        /// Use manual (RETURN-driven) mode instead of automatic ticks
        #[arg(short, long)]
        manual: bool,

        /// Tick duration in milliseconds (automatic mode only)
        #[arg(short = 'd', long, default_value = "100")]
        tick_duration_ms: u64,
    },
}

fn setup_logging() {
    // SAFETY: those are valid directives
    #[allow(clippy::unwrap_used)]
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
        .from_env()
        .unwrap();

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logging();

    let cli = Cli::parse();

    match cli.command {
        Commands::InitTopology { nodes, output } => {
            info!("Generating topology with {nodes} nodes");
            let topology: Vec<TopologyNode> = (0..nodes)
                .map(|id| {
                    let addr = SocketAddr::from(([127, 0, 0, 1], 9000 + id as u16));
                    TopologyNode::new(id, 100, addr)
                })
                .collect();
            let json = serde_json::to_string_pretty(&topology)?;
            std::fs::write(&output, &json)?;
            info!("Topology written to {output}");
        }
        Commands::Run {
            topology,
            manual,
            tick_duration_ms,
        } => {
            info!("Loading topology from {topology}");
            let driver = MixSimDriver::<u32, SimplePacket>::new(topology)?;
            info!("MixSimDriver initialized successfully");
            driver.run(manual, tick_duration_ms).await?;
        }
    }

    Ok(())
}
