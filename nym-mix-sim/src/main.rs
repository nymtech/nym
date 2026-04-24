// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Binary entry point for the `mix-sim` CLI tool.
//!
//! Provides two subcommands:
//!
//! * **`init-topology`** — generate a `topology.json` file describing N
//!   localhost mix nodes and one client, with sequential UDP ports.
//! * **`run`** — load a topology, spin up a [`SimpleMixDriver`], and drive the
//!   simulation until Ctrl-C.  Supports automatic tick mode (configurable
//!   interval via `--tick-duration-ms`) or manual RETURN-driven stepping
//!   (`--manual`).  Use the standalone `client` binary to inject packets while
//!   the simulation is running.

use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use mix_sim::{
    driver::SimpleMixDriver,
    topology::{Topology, TopologyClient, TopologyNode},
};
use tracing::info;

#[derive(Parser)]
#[command(name = "mix-sim", about = "Nym mix network simulator")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a topology.json file with a given number of nodes and one client
    InitTopology {
        /// Number of mix nodes to generate.
        ///
        /// Each node receives an auto-assigned ID (0..N-1) and a sequential
        /// localhost address starting at `127.0.0.1:9000`.
        #[arg(short, long, default_value_t = 6)]
        nodes: u8,

        /// Number of clients to generate.
        ///
        /// Each client receives an auto-assigned ID (`N..N+C`) and two
        /// sequential localhost addresses: a mix-network socket starting at
        /// `127.0.0.1:9500` and an app socket starting at `127.0.0.1:9600`.
        #[arg(short, long, default_value_t = 2)]
        clients: u8,

        /// Output file path
        #[arg(short, long, default_value = "topology.json")]
        output: String,
    },

    /// Run the mix simulation with a given topology file
    Run {
        /// Path to the topology.json file
        #[arg(short, long, default_value = "topology.json")]
        topology: String,

        /// Use manual (RETURN-driven) mode instead of automatic ticks.
        #[arg(short, long)]
        manual: bool,

        /// Tick duration in milliseconds (automatic mode only).
        #[arg(short = 'd', long, default_value = "100")]
        tick_duration_ms: u64,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    nym_bin_common::logging::setup_tracing_logger();

    let cli = Cli::parse();

    match cli.command {
        Commands::InitTopology {
            nodes,
            clients,
            output,
        } => {
            info!("Generating topology with {nodes} node(s) and {clients} client(s)");
            let node_list = (0..nodes)
                .map(|id| {
                    let addr = SocketAddr::from(([127, 0, 0, 1], 9000 + id as u16));
                    TopologyNode::new(id, 100, addr)
                })
                .collect();
            // Client binds to the next port after all nodes.
            let client_list = (nodes..nodes + clients)
                .map(|id| {
                    let mix_addr = SocketAddr::from(([127, 0, 0, 1], 9500 + id as u16));
                    let app_addr = SocketAddr::from(([127, 0, 0, 1], 9600 + id as u16));
                    TopologyClient::new(id, mix_addr, app_addr)
                })
                .collect();
            let topology = Topology {
                nodes: node_list,
                clients: client_list,
            };
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
            let driver = SimpleMixDriver::new(topology)?;
            info!("MixSimDriver initialized successfully");

            driver.run(manual, tick_duration_ms).await?;
        }
    }

    Ok(())
}
