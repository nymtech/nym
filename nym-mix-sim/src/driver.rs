// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Top-level simulation orchestrator.
//!
//! [`MixSimDriver`] owns the complete list of [`Node`]s and is the single entry
//! point for running the simulation.  It is responsible for:
//!
//! 1. **Bootstrapping** — loading `topology.json`, binding UDP sockets, and
//!    distributing the shared [`Directory`] to every node.
//! 2. **Ticking** — advancing every node through the three phases of a
//!    simulation step (incoming → processing → outgoing).
//! 3. **Driving** — either automatically (sleeping between ticks) or manually
//!    (waiting for the user to press ENTER).

use std::{fmt::Debug, sync::Arc, time::Duration};

use anyhow::Context;
use tracing::{debug, info};

use nym_lp_data::mixnodes::traits::MixnodeProcessingPipeline;

use crate::{
    node::Node,
    packet::WirePacketFormat,
    topology::{
        TopologyNode,
        directory::{Directory, NodeId},
    },
};

/// Top-level orchestrator for the mix-network simulation.
///
/// Holds an ordered list of [`Node`]s corresponding to the entries in the
/// topology file.
///
/// `Ts` is the tick-context / timestamp type; `Pkt` is the packet type.  The
/// concrete instantiation used by `main.rs` is `MixSimDriver<u32, SimplePacket>`.
///
pub struct MixSimDriver<Ts, Pkt> {
    /// All simulation nodes
    nodes: Vec<Node<Ts, Pkt>>,
}

impl<Ts, Pkt> MixSimDriver<Ts, Pkt>
where
    Ts: Debug,
    Pkt: Debug,
{
    /// Load a topology from `topology_file_path` and initialise all nodes.
    ///
    /// ## Startup sequence
    ///
    /// 1. Read and parse the JSON topology file into a `Vec<TopologyNode>`.
    /// 2. For each entry, create a [`Node`] and bind its UDP socket.
    /// 3. Build a shared [`Directory`] from the fully-bound nodes.
    /// 4. Distribute the [`Directory`] (via [`Arc`]) to every node.
    ///
    /// The [`Directory`] must be built *after* all sockets are bound so that
    /// every node's address is already finalised when the directory is
    /// populated.
    ///
    /// # Errors
    ///
    /// Returns an error if the topology file cannot be read, if the JSON is
    /// malformed, or if any node fails to bind its UDP socket.
    pub fn new<Pb, P>(topology_file_path: String, pipeline_builder: Pb) -> anyhow::Result<Self>
    where
        Pb: Fn(&TopologyNode) -> P,
        P: MixnodeProcessingPipeline<Ts, Pkt, NodeId> + Send + 'static,
    {
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
        for topology_node in topology {
            let pipeline = pipeline_builder(&topology_node);
            nodes.push(Node::new(topology_node, pipeline)?);
        }

        // 3. Build Directory
        let directory = Arc::new(Directory::build_from_nodes(&nodes));

        // 4. Give Directory to nodes
        for node in &mut nodes {
            node.set_directory(directory.clone());
        }

        Ok(Self { nodes })
    }

    /// Pretty-print the current state of every node at the given `tick` number.
    ///
    /// Wraps each node's [`display_state`] output inside a Unicode box with a
    /// tick-number header, producing output like:
    ///
    /// ```text
    /// ┌─── Tick 3 ─────────────────────────────────────┐
    /// │  Node  0 @ 127.0.0.1:9000
    /// │    to_process buffer: (empty)
    /// │    processed buffer: (empty)
    /// |----------------------
    /// └──────────────────────────────────────────────────┘
    /// ```
    ///
    /// Called by [`tick`] when `display_state` is `true`.
    ///
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
/// If a richer timestamp type is needed in the future, a new impl block should be done.
impl<Pkt> MixSimDriver<u32, Pkt>
where
    Pkt: WirePacketFormat + Debug,
{
    /// Start the simulation in either manual or automatic mode.
    ///
    /// Delegates to [`run_manual`] or [`run_automatic`] depending on the
    /// `manual_mode` flag.  `tick_duration_ms` is forwarded to
    /// [`run_automatic`] and ignored in manual mode.
    ///
    /// # Errors
    ///
    /// Propagates any error returned by the chosen run mode (I/O errors for
    /// STDIN in manual mode, Ctrl-C signal errors in automatic mode).
    ///
    pub async fn run(self, manual_mode: bool, tick_duration_ms: u64) -> anyhow::Result<()> {
        if manual_mode {
            self.run_manual().await
        } else {
            self.run_automatic(tick_duration_ms).await
        }
    }

    /// Run the simulation automatically, advancing one tick every
    /// `tick_duration_ms` milliseconds until Ctrl-C is received.
    ///
    /// The simulation loop runs inside a `tokio::spawn`-ed task so that the
    /// main task can await the Ctrl-C signal independently.  When the signal
    /// arrives the spawned task is aborted (in-flight tick work is discarded).
    ///
    /// State is *not* displayed in automatic mode (the `display_state` flag is
    /// `false`) to avoid flooding the terminal.
    ///
    /// # Errors
    ///
    /// Returns an error if the Ctrl-C signal handler cannot be installed.
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
    ///
    /// After each tick the full node state is displayed so the user can
    /// inspect packet buffers at each step.  The loop runs until the process
    /// is killed (e.g. Ctrl-C).
    ///
    /// # Errors
    ///
    /// Returns an error if reading from `stdin` fails.
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
    /// A tick is composed of three sequential phases, each applied globally
    /// across all nodes before the next phase begins.  This ensures that
    /// packets sent by nodes during `tick_outgoing` are not received by nodes
    /// in the *same* tick's `tick_incoming` — they arrive in the *next*
    /// tick, modelling network propagation delay.
    ///
    /// ## Phases
    ///
    /// 1. **Incoming** — every node drains its UDP socket into `packets_to_process`.
    /// 2. *(optional state display)*
    /// 3. **Processing** — every node mixes all packets in `packets_to_process`
    ///    into `processed_packets`.
    /// 4. *(optional state display)*
    /// 5. **Outgoing** — every node forwards all packets in `processed_packets`
    ///    to `node_id + 1`.
    ///
    /// If `display_state` is `true`, the node buffers are printed after the
    /// incoming phase (so you can see what arrived) and again after processing
    /// (so you can see the mixed result before it departs).
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
