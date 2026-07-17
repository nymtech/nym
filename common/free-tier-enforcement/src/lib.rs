// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Free-tier datapath enforcement for `nym-node` (Linux-only): the shared `tc`
//! rate-limit pool and (task 5) the `iptables` walled-garden managers. `nym-node`
//! wires these in; the network-namespace integration tests under `tests/` exercise
//! the datapath against a real kernel.
//!
//! Managers emit [`CommandSpec`]s that a [`CommandRunner`] executes - the split keeps
//! command generation unit-testable without root, while the netns harness validates
//! the live behaviour. [`RateLimitPool`] (task 4) and [`WalledGarden`] (task 5) share
//! the same command/runner/[`PeerAddrs`] seams.

use std::net::{Ipv4Addr, Ipv6Addr};

pub use command::{CommandRunner, CommandSpec, SystemCommandRunner};
pub use error::EnforcementError;
pub use garden::WalledGarden;
pub use tc::RateLimitPool;

mod command;
mod error;
mod garden;
mod iptables;
mod tc;

/// A free peer's dual-stack tunnel addresses (its WireGuard allowed IPs). Both
/// families are always present, so enforcement rules are added and removed as a pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerAddrs {
    pub v4: Ipv4Addr,
    pub v6: Ipv6Addr,
}
