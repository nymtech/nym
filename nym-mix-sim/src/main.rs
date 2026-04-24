// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Binary entry point for the `mix-sim` CLI tool.
//!
//! Provides two subcommands:
//!
//! * **`init-topology`** — generate a `topology.json` file describing N
//!   localhost mix nodes with sequential UDP ports starting at 9000.
//! * **`run`** — load a topology, spin up a [`MixSimDriver`], inject one
//!   initial packet, and drive the simulation until Ctrl-C.

use std::net::{SocketAddr, UdpSocket};

use clap::{Parser, Subcommand};
use mix_sim::{
    driver::MixSimDriver,
    packet::{SimplePacket, SimplePassThroughPipeline},
    topology::TopologyNode,
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
    /// Generate a topology.json file with a given number of nodes
    InitTopology {
        /// Number of nodes to generate.
        ///
        /// Each node receives an auto-assigned ID (0..N-1) and a sequential
        /// localhost address starting at `127.0.0.1:9000`
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

        /// Use manual (RETURN-driven) mode instead of automatic ticks.
        ///
        /// In manual mode the simulation pauses after each tick and waits for
        /// the user to press ENTER.  Node buffer state is displayed on every
        /// tick so the user can inspect packet propagation step by step.
        #[arg(short, long)]
        manual: bool,

        /// Tick duration in milliseconds (automatic mode only).
        ///
        /// Ignored when `--manual` is set.  Lower values produce faster
        /// simulation but give less time for in-flight UDP datagrams to be
        /// delivered between ticks.
        #[arg(short = 'd', long, default_value = "100")]
        tick_duration_ms: u64,
    },
}

/// Async entry point.
///
/// Parses CLI arguments, then dispatches to the appropriate handler:
///
/// * [`Commands::InitTopology`] — serialises a fresh node list to JSON.
/// * [`Commands::Run`] — bootstraps the driver, then runs the
///   simulation loop until Ctrl-C.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    nym_bin_common::logging::setup_tracing_logger();

    let cli = Cli::parse();

    match cli.command {
        Commands::InitTopology { nodes, output } => {
            info!("Generating topology with {nodes} nodes");
            // Assign sequential IDs and ports: node 0 → :9000, node 1 → :9001, …
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
            let driver = MixSimDriver::<u32, SimplePacket>::new(topology, |top_node| {
                SimplePassThroughPipeline::new(top_node.node_id.wrapping_add(1))
            })?;
            info!("MixSimDriver initialized successfully");
            let init_packet = SimplePacket::new([b'A'; 48]);
            let socket = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 9999)))?;
            socket.send_to(
                &init_packet.to_bytes(),
                SocketAddr::from(([127, 0, 0, 1], 9000)),
            )?;

            driver.run(manual, tick_duration_ms).await?;
        }
    }

    Ok(())
}
