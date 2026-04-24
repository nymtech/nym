// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Standalone client CLI — inject packets into a running mix-sim.
//!
//! Reads lines from stdin and, on each ENTER, wraps the text in a
//! [`SimplePacket`] and sends it to the running client identified by `--src`.
//! The client forwards the packet into the mix network toward node `--dst`.
//!
//! ## Message format
//!
//! ```text
//! ┌─────────────────────┬──────────────────────────────┐
//! │  dst_node_id (1 B)  │  SimplePacket wire bytes     │
//! └─────────────────────┴──────────────────────────────┘
//! ```
//!
//! The running client's `tick_incoming` parses this datagram on the next tick.
//!
//! ## Usage
//!
//! ```text
//! cargo run --bin client -- --topology topology.json --src 6 --dst 0
//! ```

use std::net::UdpSocket;

use clap::Parser;
use mix_sim::topology::{ClientId, Topology, directory::NodeId};

#[derive(Parser)]
#[command(name = "client", about = "Send stdin lines into a running mix-sim")]
struct Cli {
    /// Path to the topology.json file.
    #[arg(short, long, default_value = "topology.json")]
    topology: String,

    /// ID of the client (in the topology) to deliver packets through.
    #[arg(short, long)]
    src: ClientId,

    /// ID of the mix-node to address packets to.
    #[arg(short, long)]
    dst: NodeId,
}

fn main() -> anyhow::Result<()> {
    nym_bin_common::logging::setup_tracing_logger();

    let cli = Cli::parse();

    let topology_data = std::fs::read_to_string(&cli.topology)?;
    let topology: Topology = serde_json::from_str(&topology_data)?;

    let client = topology
        .clients
        .iter()
        .find(|c| c.client_id == cli.src)
        .ok_or_else(|| anyhow::anyhow!("no client with id {}", cli.src))?;

    let app_addr = client.app_address;

    // Bind an ephemeral socket to send from.
    let socket = UdpSocket::bind("127.0.0.1:0")?;

    println!(
        "Ready — type a message and press ENTER to send to node {} via client {}.",
        cli.dst, cli.src
    );
    println!("(Ctrl-C to quit)");

    let mut line = String::new();
    loop {
        line.clear();
        if std::io::stdin().read_line(&mut line)? == 0 {
            break; // EOF
        }

        let text = line.trim_end_matches('\n').trim_end_matches('\r');
        let bytes = text.as_bytes();

        // Prepend the destination node ID.
        let mut msg = Vec::with_capacity(1 + bytes.len());
        msg.push(cli.dst);
        msg.extend_from_slice(bytes);

        socket.send_to(&msg, app_addr)?;
        println!(
            "Sent {} byte(s) of payload to client {} → node {}.",
            bytes.len(),
            cli.src,
            cli.dst
        );
    }

    Ok(())
}
