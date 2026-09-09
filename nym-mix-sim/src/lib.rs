// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! # nym-mix-sim
//!
//! A discrete-time simulator for a Nym mixnet, intended for local testing and
//! experimentation. The simulator models a network of mix nodes exchanging packets.
//!
//! It runs two ways, over one core. A live run binds a UDP socket per participant and is driven
//! from the CLI, with `mix-client` injecting payloads and deliveries logged. A seeded run puts the
//! whole network in process, and is reproducible in route, order and delivery so a test can assert
//! on it - see [`sim::SimHarness`]. Which one is in play is decided entirely by the
//! [`sim::env::SimEnv`] handed to a driver's `build`.
//!
//! ## Architecture overview
//!
//! ```text
//!  ┌──────────────┐      JSON      ┌──────────────────────────────┐
//!  │ topology.json│ ─────────────▶ │ MixSimDriver                 │
//!  └──────────────┘                │  ├─ one node per topology    │
//!                                  │  │    entry, on its own IP   │
//!                                  │  └─ one client per topology  │
//!                                  │       entry, mix + app addr  │
//!                                  └──────────────────────────────┘
//!
//!  Every participant reaches the others through a `SimEndpoint`: a bound UDP
//!  socket in a live run, an in-process inbox in a seeded one. Each needs a
//!  distinct IP either way, since LP sessions between nodes are keyed by it.
//!
//!  Each simulation tick:
//!    1. client tick     – every client drains its app socket, queues outgoing
//!                         packets, and processes inbound mix packets
//!    2. tick_incoming   – every node drains its endpoint into an inbound buffer
//!    3. tick_processing – every node transforms buffered packets (mix operation)
//!    4. tick_outgoing   – every node forwards processed packets to the next hop
//! ```
//!
//! ## Crate layout
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`driver`]   | Top-level orchestrator; owns all nodes and clients, drives simulation ticks |
//! | [`node`]     | Individual mix node: endpoint, inbound/outbound packet buffers |
//! | [`client`]   | Simulated client: injects application payloads into the mix network |
//! | [`packet`]   | Wire format types and the [`packet::WirePacketFormat`] trait |
//! | [`topology`] | Topology file types and the in-memory [`topology::directory::Directory`] |
//! | [`transport`]| [`transport::SimEndpoint`]: a real socket, or an in-process switchboard |
//! | [`logging`]  | [`logging::SimLogging`]: what becomes of a message once it arrives |
//! | [`sim`]      | Choosing between the two, and the test harness built on the seeded one |

pub mod client;
pub mod driver;
pub mod helpers;
pub mod logging;
pub mod node;
pub mod packet;
pub mod peers;
pub mod sim;
pub mod topology;
pub mod transport;
