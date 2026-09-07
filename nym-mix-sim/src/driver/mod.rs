// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Top-level simulation orchestrator.
//!
//! [`MixSimDriver`] owns the complete list of [`MixSimNode`]s and
//! [`MixSimClient`]s and is the single entry point for running the simulation.
//! It is responsible for:
//!
//! 1. **Bootstrapping** — building the shared [`Directory`](crate::topology::directory::Directory)
//!    from pre-constructed nodes and clients, then distributing it to every participant.
//! 2. **Ticking** — advancing every node and client through the phases of a
//!    simulation step (client tick → incoming → processing → outgoing).
//! 3. **Driving** — either automatically (sleeping between ticks) or manually
//!    (waiting for the user to press ENTER).
//!
//! Nodes and clients are built externally (e.g. in [`SimpleMixDriver`]) and
//! passed to [`MixSimDriver::new`] as boxed trait objects, so the driver only
//! needs to know the timestamp type `Ts`.
//!
//! To inject packets into a running simulation, use the standalone `mix-client`
//! binary, which sends payloads to a client's app socket.

use std::{
    fmt::Debug,
    time::{Duration, Instant},
};

use tracing::info;

use crate::{client::MixSimClient, node::MixSimNode};

mod nymnode;
mod simple;
mod sphinx;

pub use nymnode::NymNodeMixDriver;
pub use simple::SimpleMixDriver;
pub use sphinx::SphinxMixDriver;

/// Top-level orchestrator for the mix-network simulation.
///
/// Holds ordered lists of type-erased [`MixSimNode`]s and [`MixSimClient`]s.
/// Only the timestamp type `Ts` is visible at this level; packet format, frame
/// type, and message marker are encapsulated inside each concrete node/client.
pub struct MixSimDriver {
    nodes: Vec<Box<dyn MixSimNode + Send>>,
    clients: Vec<Box<dyn MixSimClient + Send>>,
    clock_base: Instant,
}

impl MixSimDriver {
    /// Construct the driver from pre-built nodes and clients.
    ///
    /// Topology parsing and socket binding are the caller's responsibility.
    pub fn new(
        nodes: Vec<Box<dyn MixSimNode + Send>>,
        clients: Vec<Box<dyn MixSimClient + Send>>,
    ) -> Self {
        Self {
            nodes,
            clients,
            clock_base: Instant::now(),
        }
    }

    pub fn display_tick(&self, tick: Instant) -> u128 {
        tick.duration_since(self.clock_base).as_millis()
    }

    /// Pretty-print the current state of every node at `tick`.
    pub fn display_state(&self, tick: Instant) {
        println!(
            "┌─── Tick {:─<3} ms─────────────────────────────────────────────────────────┐",
            self.display_tick(tick)
        );
        for node in &self.nodes {
            node.display_state();
            println!("|------------------------------------------------------------------------|")
        }
        println!("└────────────────────────────────────────────────────────────────────────┘");
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
    pub fn tick(&mut self, timestamp: Instant, display_state: bool) {
        for client in &mut self.clients {
            client.tick(timestamp);
        }
        // Phase 1 — incoming
        for node in &mut self.nodes {
            node.tick_incoming();
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

    /// Start the simulation in either manual or automatic mode.
    pub async fn run(
        self,
        manual_mode: bool,
        display_state: bool,
        tick_duration_ms: u64,
    ) -> anyhow::Result<()> {
        if manual_mode {
            self.run_manual(tick_duration_ms, display_state)
        } else {
            self.run_automatic(tick_duration_ms).await
        }
    }

    /// Run the simulation automatically, advancing one tick every
    /// `tick_duration_ms` milliseconds until Ctrl-C is received.
    pub async fn run_automatic(mut self, tick_duration_ms: u64) -> anyhow::Result<()> {
        info!("Automatic mode: tick duration : {tick_duration_ms} ms");
        let tick_duration = Duration::from_millis(tick_duration_ms);
        let handle = tokio::spawn(async move {
            loop {
                let current_tick = Instant::now();
                self.tick(current_tick, false);
                tokio::time::sleep(tick_duration).await;
            }
        });
        tokio::signal::ctrl_c().await?;
        handle.abort();
        Ok(())
    }

    /// Run the simulation interactively: one tick per ENTER key press.
    pub fn run_manual(mut self, tick_duration_ms: u64, display_state: bool) -> anyhow::Result<()> {
        info!("Manual mode: press ENTER to advance a tick, Ctrl-C to quit");
        info!("One tick represent {tick_duration_ms}ms");
        let tick_duration = Duration::from_millis(tick_duration_ms);
        let mut current_tick = self.clock_base;
        let mut line = String::new();
        loop {
            line.clear();
            std::io::stdin().read_line(&mut line)?;
            info!("Tick {}ms", self.display_tick(current_tick));
            self.tick(current_tick, display_state);
            current_tick += tick_duration;
        }
    }
}

/// Which simulation driver to use.
#[derive(Clone, Debug, Default, strum::Display, strum::EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum SimDriver {
    /// Simple pass-through packets.
    Simple,
    /// Full Sphinx encryption with SURBACKs and cover traffic
    Sphinx,
    /// Real [`NymNodeDataPipeline`] processing sphinx-in-LP packets.
    ///
    /// [`NymNodeDataPipeline`]: nym_node::node::lp::data::handler::pipeline::NymNodeDataPipeline
    #[default]
    NymNode,
}

impl SimDriver {
    /// Dispatch to the appropriate concrete driver and start the simulation.
    pub async fn run(
        self,
        topology: String,
        manual: bool,
        display_state: bool,
        tick_duration_ms: u64,
    ) -> anyhow::Result<()> {
        match self {
            SimDriver::Simple => {
                SimpleMixDriver::new(topology)?
                    .run(manual, display_state, tick_duration_ms)
                    .await
            }
            SimDriver::Sphinx => {
                SphinxMixDriver::new(topology)?
                    .run(manual, display_state, tick_duration_ms)
                    .await
            }
            SimDriver::NymNode => {
                NymNodeMixDriver::new(topology)
                    .await?
                    .run(manual, display_state, tick_duration_ms)
                    .await
            }
        }
    }
}
