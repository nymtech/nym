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

use crate::{
    client::MixSimClient,
    node::MixSimNode,
    sim::env::LiveEnv,
    topology::{NodeRole, Topology},
};

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

    /// Draw the network as a packet crosses it: the route, then what each leg is holding.
    ///
    /// Legs rather than nodes, because the route is the story and a node's identity is not. Every
    /// node of a role is summed into its leg, so the shape stays the same whether the topology has
    /// four nodes or forty.
    pub fn display_state(&self, tick: Instant, phase: &str) {
        let nodes: Vec<_> = self.nodes.iter().map(|node| node.snapshot()).collect();
        let clients: Vec<_> = self
            .clients
            .iter()
            .map(|client| client.snapshot())
            .collect();

        /// Enough to see the shape of what a leg is carrying without the frame running off-screen.
        const MOST: usize = 3;

        println!();
        println!(
            "  Nym mixsim · step {} ms · {phase}",
            self.display_tick(tick)
        );
        println!();
        println!(
            "  route   client ─▶ gateway ─▶ layer 1 ─▶ layer 2 ─▶ layer 3 ─▶ gateway ─▶ client"
        );
        println!();
        // a gateway is both ends of a route, so it is one leg holding both lots of traffic
        for (label, role) in [
            ("gateways", NodeRole::Gateway),
            ("layer 1 ", NodeRole::Layer1),
            ("layer 2 ", NodeRole::Layer2),
            ("layer 3 ", NodeRole::Layer3),
        ] {
            let leg: Vec<_> = nodes.iter().filter(|node| node.role == role).collect();
            let held: usize = leg
                .iter()
                .map(|node| node.sealed.len() + node.opened.len())
                .sum();

            println!("  {label}  {}  {held}", packet_bar(held));

            // What it is holding but has not opened, it can read no more of than the envelope.
            // What it has opened turned out to be LP framing wrapped around a sphinx packet whose
            // contents it still cannot read.
            for line in leg.iter().flat_map(|node| node.sealed.iter()).take(MOST) {
                println!("      in    {line}");
            }
            for frame in leg.iter().flat_map(|node| node.opened.iter()).take(MOST) {
                // the wait is the mixing delay: the node is deliberately sitting on this so that
                // when it leaves says nothing about when it arrived. Both the wait and the tick it
                // lands on, so a viewer can watch for that exact frame rather than count steps.
                let wait = frame.release.saturating_duration_since(tick).as_millis();
                let at = self.display_tick(frame.release);
                let due = if wait == 0 {
                    format!("leaving now        (tick {at})")
                } else {
                    format!("leaves in {wait:>3} ms   (tick {at})")
                };

                // wide enough for the longest a frame gets - "FragmentedData 1/2 → SphinxPacket"
                // plus its size - so the times stay in a column
                println!("      held  {:<42}  {due}", frame.what);
            }
        }

        println!();
        println!("  clients");
        for client in &clients {
            let sending = if client.outbox.is_empty() {
                "idle".to_string()
            } else {
                format!("sending {}", client.outbox.len())
            };
            println!("    client {}   {sending}", client.id);

            // sealed here, before the first hop has them: the same view every node downstream
            // gets, which is the point - nobody past this line sees any more than this
            for packet in client.outbox.iter().take(MOST) {
                println!("        ▶ {packet}");
            }
        }
        println!();
    }

    /// Advance the simulation by one tick.
    ///
    /// ## Phases
    ///
    /// 1. **Client**  - clients tick.
    /// 2. **Incoming** — every node drains its endpoint into `packets_to_process`.
    /// 3. *(optional state display)*
    /// 4. **Processing** — every node mixes buffered packets.
    /// 5. *(optional state display)*
    /// 6. **Outgoing** — nodes forward due packets;
    pub fn tick(&mut self, timestamp: Instant, display_state: bool) {
        // Phase 1 — clients take in what is being sent, and deliver what came back
        for client in &mut self.clients {
            client.tick_incoming(timestamp);
        }

        // Phase 2 — incoming
        for node in &mut self.nodes {
            node.tick_incoming();
        }

        if display_state {
            self.display_state(timestamp, "collected");
        }

        // Phase 3 — processing
        for node in &mut self.nodes {
            node.tick_processing(timestamp);
        }

        if display_state {
            self.display_state(timestamp, "mixed");
        }

        // Phase 4 — outgoing
        for node in &mut self.nodes {
            node.tick_outgoing(timestamp);
        }
        for client in &mut self.clients {
            client.tick_outgoing(timestamp);
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
    ///
    /// Always a [`LiveEnv`]: which driver runs is picked at the command line, so this is no use to
    /// a test harness, which builds one concrete driver against a seeded environment instead.
    pub async fn run(
        self,
        topology: String,
        manual: bool,
        display_state: bool,
        tick_duration_ms: u64,
    ) -> anyhow::Result<()> {
        let topology = Topology::load(&topology)?;
        match self {
            SimDriver::Simple => {
                SimpleMixDriver::new(topology, &mut LiveEnv)?
                    .run(manual, display_state, tick_duration_ms)
                    .await
            }
            SimDriver::Sphinx => {
                SphinxMixDriver::new(topology, &mut LiveEnv)?
                    .run(manual, display_state, tick_duration_ms)
                    .await
            }
            SimDriver::NymNode => {
                NymNodeMixDriver::new(topology, &mut LiveEnv)
                    .await?
                    .run(manual, display_state, tick_duration_ms)
                    .await
            }
        }
    }
}

/// A count as something you can see the size of at a glance.
///
/// Capped, because the point is "a little" against "a lot" - the number beside it is there for
/// anyone who wants the exact figure.
fn packet_bar(count: usize) -> String {
    const WIDEST: usize = 10;

    if count == 0 {
        return format!("{:<WIDEST$}", "·");
    }

    let filled = count.min(WIDEST);
    let overflow = if count > WIDEST { "+" } else { "" };
    format!("{:<WIDEST$}", format!("{}{overflow}", "▓".repeat(filled)))
}
