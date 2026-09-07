// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Binary entry point for the `nym-mix-sim` CLI tool.
//!
//! Provides two subcommands:
//!
//! * **`init-topology`** — generate a `topology.json` file describing N
//!   localhost mix nodes and C clients, with sequential UDP ports.
//! * **`run`** — load a topology, spin up the chosen [`SimDriver`], and drive
//!   the simulation until Ctrl-C.  Supports automatic tick mode (configurable
//!   interval via `--tick-duration-ms`) or manual RETURN-driven stepping
//!   (`--manual`).  Use the standalone `mix-client` binary to inject packets
//!   while the simulation is running.

use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use nym_mix_sim::{
    driver::SimDriver,
    topology::{MIN_NODES, Topology, TopologyClient, TopologyNode},
};
use tracing::info;

#[derive(Parser)]
#[command(name = "nym-mix-sim", about = "Nym mix network simulator")]
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
        #[arg(short, long, default_value_t = 3)]
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

        /// Suppress node state display after each tick phase (manual mode only).
        #[arg(long)]
        no_display_state: bool,

        /// Tick duration in milliseconds.
        #[arg(short = 'd', long, default_value = "10")]
        tick_duration_ms: u64,

        /// Simulation driver to use: simple | sphinx | nym-node (default).
        #[arg(long, default_value_t = SimDriver::NymNode)]
        driver: SimDriver,
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
            if (nodes as usize) < MIN_NODES {
                anyhow::bail!("a simulation needs at least {MIN_NODES} nodes, {nodes} requested");
            }

            info!("Generating topology with {nodes} node(s) and {clients} client(s)");
            let node_list = (0..nodes)
                .map(|id| {
                    let node_id = id + 1;
                    let addr = SocketAddr::from(([127, 0, 0, node_id], 51264));
                    TopologyNode::new(node_id, 100, addr)
                })
                .collect();
            // Client binds to the next port after all nodes.
            let client_list = (nodes..nodes + clients)
                .map(|id| {
                    let client_id = id + 1;
                    let mix_addr = SocketAddr::from(([127, 0, 0, client_id], 9000));
                    let app_addr = SocketAddr::from(([127, 0, 0, client_id], 9001));
                    TopologyClient::new(client_id, mix_addr, app_addr)
                })
                .collect();
            let topology = Topology {
                nodes: node_list,
                clients: client_list,
            };
            let json = serde_json::to_string_pretty(&topology)?;
            std::fs::write(&output, &json)?;
            info!("Topology written to {output}");
            info!(
                "Make sure addresses from 127.0.0.1 until 127.0.0.{} are bound to an interface",
                nodes + clients
            )
        }
        Commands::Run {
            topology,
            manual,
            no_display_state,
            tick_duration_ms,
            driver,
        } => {
            info!("Loading topology from {topology} with driver={driver}");
            driver
                .run(topology, manual, !no_display_state, tick_duration_ms)
                .await?;
        }
    }

    Ok(())
}
