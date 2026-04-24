// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Parser, Subcommand};
use mix_sim::{driver::MixSimDriver, node::TopologyNode};
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
    },
}

fn setup_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

fn main() -> anyhow::Result<()> {
    setup_logging();

    let cli = Cli::parse();

    match cli.command {
        Commands::InitTopology { nodes, output } => {
            info!("Generating topology with {nodes} nodes");
            let topology: Vec<TopologyNode> =
                (0..nodes).map(|id| TopologyNode::new(id, 100)).collect();
            let json = serde_json::to_string_pretty(&topology)?;
            std::fs::write(&output, &json)?;
            info!("Topology written to {output}");
        }
        Commands::Run { topology } => {
            info!("Loading topology from {topology}");
            let _driver = MixSimDriver::<(), ()>::new(topology)?;
            info!("MixSimDriver initialized successfully");
        }
    }

    Ok(())
}
