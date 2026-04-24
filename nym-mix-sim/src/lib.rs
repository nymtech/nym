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
//!  ┌──────────────┐      JSON      ┌──────────────────────────────┐
//!  │ topology.json│ ─────────────▶ │ MixSimDriver                 │
//!  └──────────────┘                │  ├─ Node 0 (UDP :9000)       │
//!                                  │  ├─ Node 1 (UDP :9001)       │
//!                                  │  └─ Node N (UDP :900N)       │
//!                                  └──────────────────────────────┘
//!
//!  Each simulation tick:
//!    1. tick_incoming  – every node drains its UDP socket into an inbound buffer
//!    2. tick_processing – every node transforms buffered packets (mix operation)
//!    3. tick_outgoing  – every node forwards processed packets to the next hop
//! ```
//!
//! ## Crate layout
//!
//! | Module | Purpose |
//! |--------|---------|
//! [`driver`] | Top-level orchestrator; owns all nodes and drives simulation ticks |
//! [`node`]   | Individual mix node: UDP socket, inbound/outbound packet buffers |
//! [`packet`] | Wire format types and the [`packet::WirePacketFormat`] trait |
//! [`topology`] | In-memory directory mapping [`topology::directory::NodeId`] → address |

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

pub mod driver;
pub mod node;
pub mod packet;
pub mod topology;
