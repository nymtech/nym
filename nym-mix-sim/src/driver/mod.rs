// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Top-level simulation orchestrator.
//!
//! [`MixSimDriver`] owns the complete list of [`Node`]s and [`Client`]s and is
//! the single entry point for running the simulation.  It is responsible for:
//!
//! 1. **Bootstrapping** — loading `topology.json`, binding UDP sockets, and
//!    distributing the shared [`Directory`] to every node and client.
//! 2. **Ticking** — advancing every node and client through the phases of a
//!    simulation step (client tick → incoming → processing → outgoing).
//! 3. **Driving** — either automatically (sleeping between ticks) or manually
//!    (waiting for the user to press ENTER).
//!
//! To inject packets into a running simulation, use the standalone `client`
//! binary, which sends payloads to a client's app socket.

use std::{fmt::Debug, sync::Arc, time::Duration};

use anyhow::Context;
use tracing::{debug, info};

use nym_lp_data::clients::traits::{ClientUnwrappingPipeline, DynProcessingPipeline};
use nym_lp_data::mixnodes::traits::MixnodeProcessingPipeline;

use crate::{
    client::Client,
    node::{Node, NodeId},
    packet::WirePacketFormat,
    topology::{Topology, TopologyClient, TopologyNode, directory::Directory},
};

mod simple;

pub use simple::SimpleMixDriver;

/// Top-level orchestrator for the mix-network simulation.
///
/// Holds ordered lists of [`Node`]s and [`Client`]s corresponding to the
/// entries in the topology file.
///
/// `Ts` is the tick-context / timestamp type; `Fr` is the intermediate frame
/// type; `Pkt` is the transport packet type.
pub struct MixSimDriver<Ts, Fr, Pkt> {
    nodes: Vec<Node<Ts, Pkt>>,
    clients: Vec<Client<Ts, Fr, Pkt>>,
}

impl<Ts, Fr, Pkt> MixSimDriver<Ts, Fr, Pkt>
where
    Ts: Debug + Clone,
    Pkt: Debug,
{
    /// Load a topology from `topology_file_path` and initialise all nodes and
    /// clients.
    ///
    /// ## Startup sequence
    ///
    /// 1. Read and parse the JSON topology file into a [`Topology`].
    /// 2. For each node entry, create a [`Node`] and bind its UDP socket.
    /// 3. For each client entry, create a [`Client`] and bind its UDP socket.
    /// 4. Build a shared [`Directory`] from the fully-bound nodes.
    /// 5. Distribute the [`Directory`] (via [`Arc`]) to every node and client.
    ///
    /// # Errors
    ///
    /// Returns an error if the topology file cannot be read, if the JSON is
    /// malformed, or if any node or client fails to bind its UDP socket.
    pub fn new<Pb, P, Cpb, Cp, Cub, Cu>(
        topology_file_path: String,
        pipeline_builder: Pb,
        client_processing_builder: Cpb,
        client_unwrapping_builder: Cub,
    ) -> anyhow::Result<Self>
    where
        Pb: Fn(&TopologyNode) -> P,
        P: MixnodeProcessingPipeline<Ts, Pkt, NodeId> + Send + 'static,
        Cpb: Fn(&TopologyClient) -> Cp,
        Cp: DynProcessingPipeline<Ts, Fr, Pkt> + Send + 'static,
        Cub: Fn(&TopologyClient) -> Cu,
        Cu: ClientUnwrappingPipeline<Ts, Pkt> + Send + 'static,
    {
        debug!("Bootstrapping from topology file: {}", topology_file_path);

        // 1. Read topology from file
        let topology_data =
            std::fs::read_to_string(&topology_file_path).context("Failed to read topology file")?;
        let topology: Topology =
            serde_json::from_str(&topology_data).context("Topology file malformed")?;

        // 2. Init nodes (bind UDP sockets)
        let mut nodes = Vec::with_capacity(topology.nodes.len());
        for topology_node in topology.nodes {
            let pipeline = pipeline_builder(&topology_node);
            nodes.push(Node::new(topology_node, pipeline)?);
        }

        // 3. Init clients (bind UDP sockets)
        let mut clients = Vec::with_capacity(topology.clients.len());
        for client_topology in topology.clients {
            let processing = client_processing_builder(&client_topology);
            let unwrapping = client_unwrapping_builder(&client_topology);
            clients.push(Client::new(client_topology, processing, unwrapping)?);
        }

        // 4. Build Directory from nodes
        let directory = Arc::new(Directory::build_from_nodes(&nodes, &clients));

        // 5. Give Directory to nodes and clients
        for node in &mut nodes {
            node.set_directory(directory.clone());
        }
        for client in &mut clients {
            client.set_directory(directory.clone());
        }

        Ok(Self { nodes, clients })
    }

    /// Pretty-print the current state of every node and client at `tick`.
    pub fn display_state(&self, tick: u32) {
        println!("┌─── Tick {tick} ─────────────────────────────────────┐");
        for node in &self.nodes {
            node.display_state();
            println!("|----------------------")
        }
        println!("└──────────────────────────────────────────────────┘");
    }
}

/// Driving logic for the concrete `Ts = u32` timestamp flavour.
///
/// The timestamp is a monotonically increasing tick counter starting at zero.
/// If a richer timestamp type is needed in the future, a new impl block should
/// be added.
impl<Fr, Pkt> MixSimDriver<u32, Fr, Pkt>
where
    Fr: Send + 'static,
    Pkt: WirePacketFormat + Debug,
{
    /// Start the simulation in either manual or automatic mode.
    pub async fn run(self, manual_mode: bool, tick_duration_ms: u64) -> anyhow::Result<()> {
        if manual_mode {
            self.run_manual().await
        } else {
            self.run_automatic(tick_duration_ms).await
        }
    }

    /// Run the simulation automatically, advancing one tick every
    /// `tick_duration_ms` milliseconds until Ctrl-C is received.
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

    /// Run the simulation interactively: one tick per ENTER key press.
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

    /// Advance the simulation by one tick.
    ///
    /// ## Phases
    ///
    /// 1. **Client**  - clients tick.
    /// 2. **Incoming** — every node drains its UDP socket into `packets_to_process`.
    /// 3. *(optional state display)*
    /// 4. **Processing** — every node mixes buffered packets.
    /// 5. *(optional state display)*
    /// 6. **Outgoing** — nodes forward due packets;
    pub async fn tick(&mut self, timestamp: u32, display_state: bool) {
        for client in &mut self.clients {
            client.tick(timestamp);
        }
        // Phase 1 — incoming
        for node in &mut self.nodes {
            node.tick_incoming(timestamp);
        }

        if display_state {
            self.display_state(timestamp);
        }

        // Phase 2 — processing
        for node in &mut self.nodes {
            node.tick_processing(timestamp);
        }

        if display_state {
            self.display_state(timestamp);
        }

        // Phase 3 — outgoing
        for node in &mut self.nodes {
            node.tick_outgoing(timestamp);
        }
    }
}
