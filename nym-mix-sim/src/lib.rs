// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! # nym-mix-sim
//!
//! A discrete-time simulator for a Nym mixnet, intended for local testing and
//! experimentation. The simulator models a network of mix nodes that exchange
//! UDP packets on localhost.
//!
//! ## Architecture overview
//!
//! ```text
//!  ┌──────────────┐      JSON      ┌───────────────────────────────┐
//!  │ topology.json│ ─────────────▶ │ MixSimDriver                  │
//!  └──────────────┘                │  ├─ Node 0 (UDP :9000)        │
//!                                  │  ├─ Node N (UDP :900N)        │
//!                                  │  ├─ Client 0 (UDP :9500/:9600)│
//!                                  │  └─ Client C (UDP :950C/:960C)│
//!                                  └───────────────────────────────┘
//!
//!  Each simulation tick:
//!    1. client tick     – every client drains its app socket, queues outgoing
//!                         packets, and processes inbound mix packets
//!    2. tick_incoming   – every node drains its UDP socket into an inbound buffer
//!    3. tick_processing – every node transforms buffered packets (mix operation)
//!    4. tick_outgoing   – every node forwards processed packets to the next hop
//! ```
//!
//! ## Crate layout
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`driver`]   | Top-level orchestrator; owns all nodes and clients, drives simulation ticks |
//! | [`node`]     | Individual mix node: UDP socket, inbound/outbound packet buffers |
//! | [`client`]   | Simulated client: injects application payloads into the mix network |
//! | [`packet`]   | Wire format types and the [`packet::WirePacketFormat`] trait |
//! | [`topology`] | Topology file types and the in-memory [`topology::directory::Directory`] |

pub mod client;
pub mod driver;
pub mod node;
pub mod packet;
pub mod topology;
