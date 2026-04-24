// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Debug, sync::Arc, time::Duration};

use anyhow::Context;
use tracing::{debug, info};

use crate::{
    node::{Node, TopologyNode},
    packet::WirePacketFormat,
    topology::Directory,
};

pub struct MixSimDriver<Ts, Pkt> {
    nodes: Vec<Node<Ts, Pkt>>,
}

impl<Ts, Pkt> MixSimDriver<Ts, Pkt>
where
    Ts: Debug,
    Pkt: Debug,
{
    pub fn new(topology_file_path: String) -> anyhow::Result<Self> {
        debug!(
            "Bootstrapping nodes from topology file: {}",
            topology_file_path
        );
        // 1. Read topology from file
        let topology_data =
            std::fs::read_to_string(&topology_file_path).context("Failed to read topolgy file")?;
        let topology: Vec<TopologyNode> =
            serde_json::from_str(&topology_data).context("Topology file malformed")?;

        // 2. Init nodes (bind UDP sockets)
        let mut nodes = Vec::with_capacity(topology.len());
        for node in topology {
            nodes.push(Node::<Ts, Pkt>::from_topology_node(node)?);
        }

        // 3. Build Directory
        let directory = Arc::new(Directory::build_from_nodes(&nodes));

        // 4. Give Directory to nodes
        for node in &mut nodes {
            node.set_directory(directory.clone());
        }

        Ok(Self { nodes })
    }

    pub fn display_state(&self, tick: u32) {
        println!("┌─── Tick {tick} ─────────────────────────────────────┐");
        for node in &self.nodes {
            node.display_state();
        }
        println!("└──────────────────────────────────────────────────┘");
    }
}

impl<Pkt> MixSimDriver<u32, Pkt>
where
    Pkt: WirePacketFormat<u32> + Debug,
{
    pub async fn run(self, manual_mode: bool, tick_duration_ms: u64) -> anyhow::Result<()> {
        if manual_mode {
            self.run_manual().await
        } else {
            self.run_automatic(tick_duration_ms).await
        }
    }

    pub async fn run_automatic(mut self, tick_duration_ms: u64) -> anyhow::Result<()> {
        let tick_duration = Duration::from_millis(tick_duration_ms);
        let handle = tokio::spawn(async move {
            let mut current_tick = 0;
            loop {
                self.tick(current_tick, false).await;
                current_tick += 1;
                tokio::time::sleep(tick_duration).await;
            }
        });
        tokio::signal::ctrl_c().await?;
        handle.abort();
        Ok(())
    }

    pub async fn run_manual(mut self) -> anyhow::Result<()> {
        info!("Manual mode: press ENTER to advance a tick, Ctrl-C to quit");
        let mut current_tick: u32 = 0;
        let mut line = String::new();
        loop {
            line.clear();
            std::io::stdin().read_line(&mut line)?;
            info!("Tick {current_tick}");
            self.tick(current_tick, true).await;
            current_tick += 1;
        }
    }

    pub async fn tick(&mut self, timestamp: u32, display_state: bool) {
        // Take in incoming packets everywhere
        for node in &mut self.nodes {
            node.tick_incoming(timestamp);
        }

        // Optionnally display state
        if display_state {
            self.display_state(timestamp);
        }
        // Process packets everywhere
        for node in &mut self.nodes {
            node.tick_processing(timestamp);
        }

        // Optionnally display state again
        if display_state {
            self.display_state(timestamp);
        }

        // Send outgoing packets everywere
        for node in &mut self.nodes {
            node.tick_outgoing(timestamp);
        }
    }
}
