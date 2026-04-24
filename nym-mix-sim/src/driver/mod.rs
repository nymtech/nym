// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Top-level simulation orchestrator.
//!
//! [`MixSimDriver`] owns the complete list of [`MixSimNode`]s and
//! [`MixSimClient`]s and is the single entry point for running the simulation.
//! It is responsible for:
//!
//! 1. **Bootstrapping** — building the shared [`Directory`] from pre-constructed
//!    nodes and clients, then distributing it to every participant.
//! 2. **Ticking** — advancing every node and client through the phases of a
//!    simulation step (client tick → incoming → processing → outgoing).
//! 3. **Driving** — either automatically (sleeping between ticks) or manually
//!    (waiting for the user to press ENTER).
//!
//! Nodes and clients are built externally (e.g. in [`SimpleMixDriver`]) and
//! passed to [`MixSimDriver::new`] as boxed trait objects, so the driver only
//! needs to know the timestamp type `Ts`.
//!
//! To inject packets into a running simulation, use the standalone `client`
//! binary, which sends payloads to a client's app socket.

use std::{
    fmt::Debug,
    time::{Duration, Instant},
};

use tracing::info;

use crate::{client::MixSimClient, node::MixSimNode};

mod simple;
mod sphinx;

pub use simple::SimpleMixDriver;
pub use sphinx::{DiscreteSphinxMixDriver, SphinxMixDriver};

/// Top-level orchestrator for the mix-network simulation.
///
/// Holds ordered lists of type-erased [`MixSimNode`]s and [`MixSimClient`]s.
/// Only the timestamp type `Ts` is visible at this level; packet format, frame
/// type, and message marker are encapsulated inside each concrete node/client.
pub struct MixSimDriver<Ts>
where
    Ts: Clone + PartialOrd + Debug + Send,
{
    nodes: Vec<Box<dyn MixSimNode<Ts> + Send>>,
    clients: Vec<Box<dyn MixSimClient<Ts> + Send>>,
}

impl<Ts> MixSimDriver<Ts>
where
    Ts: Clone + PartialOrd + Debug + Send,
{
    /// Construct the driver from pre-built nodes and clients.
    ///
    /// Topology parsing and socket binding are the caller's responsibility.
    pub fn new(
        nodes: Vec<Box<dyn MixSimNode<Ts> + Send>>,
        clients: Vec<Box<dyn MixSimClient<Ts> + Send>>,
    ) -> Self {
        Self { nodes, clients }
    }

    /// Pretty-print the current state of every node at `tick`.
    pub fn display_state(&self, tick: Ts) {
        println!("┌─── Tick {tick:?}────────────────────────────────────┐");
        for node in &self.nodes {
            node.display_state();
            println!("|----------------------")
        }
        println!("└──────────────────────────────────────────────────┘");
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
    pub async fn tick(&mut self, timestamp: Ts, display_state: bool) {
        for client in &mut self.clients {
            client.tick(timestamp.clone());
        }
        // Phase 1 — incoming
        for node in &mut self.nodes {
            node.tick_incoming(timestamp.clone());
        }

        if display_state {
            self.display_state(timestamp.clone());
        }

        // Phase 2 — processing
        for node in &mut self.nodes {
            node.tick_processing(timestamp.clone());
        }

        if display_state {
            self.display_state(timestamp.clone());
        }

        // Phase 3 — outgoing
        for node in &mut self.nodes {
            node.tick_outgoing(timestamp.clone());
        }
    }
}

/// Driving logic for the concrete `Ts = u32` timestamp flavour.
///
/// The timestamp is a monotonically increasing tick counter starting at zero.
/// If a richer timestamp type is needed in the future, a new impl block should
/// be added.
impl MixSimDriver<u32> {
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
}

/// Driving logic for the concrete `Ts = Instant` timestamp flavour.
impl MixSimDriver<Instant> {
    /// Start the simulation in either manual or automatic mode.
    pub async fn run(self, manual_mode: bool, tick_duration_ms: u64) -> anyhow::Result<()> {
        if manual_mode {
            tracing::error!("Instant-based MixSim is incompatible with manual driving mode");
            Ok(())
        } else {
            self.run_automatic(tick_duration_ms).await
        }
    }

    /// Run the simulation automatically, advancing one tick every
    /// `tick_duration_ms` milliseconds until Ctrl-C is received.
    pub async fn run_automatic(mut self, tick_duration_ms: u64) -> anyhow::Result<()> {
        let tick_duration = Duration::from_millis(tick_duration_ms);
        let handle = tokio::spawn(async move {
            let current_tick = Instant::now();
            loop {
                self.tick(current_tick, false).await;
                tokio::time::sleep(tick_duration).await;
            }
        });
        tokio::signal::ctrl_c().await?;
        handle.abort();
        Ok(())
    }
}

/// Which simulation driver to use.
#[derive(Clone, Debug, Default, strum::Display, strum::EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum SimDriver {
    /// Simple pass-through packets, discrete tick counter.
    Simple,
    /// Full Sphinx encryption, wall-clock timestamps, automatic mode only.
    Sphinx,
    /// Full Sphinx encryption, discrete tick counter, supports manual mode.
    #[default]
    ManualSphinx,
}

impl SimDriver {
    pub async fn run(
        self,
        topology: String,
        manual: bool,
        tick_duration_ms: u64,
    ) -> anyhow::Result<()> {
        match self {
            SimDriver::Simple => {
                SimpleMixDriver::new(topology)?
                    .run(manual, tick_duration_ms)
                    .await
            }
            SimDriver::Sphinx => {
                SphinxMixDriver::new(topology)?
                    .run(manual, tick_duration_ms)
                    .await
            }
            SimDriver::ManualSphinx => {
                DiscreteSphinxMixDriver::new(topology)?
                    .run(manual, tick_duration_ms)
                    .await
            }
        }
    }
}
