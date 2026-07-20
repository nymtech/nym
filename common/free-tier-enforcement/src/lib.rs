// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Free-tier datapath enforcement for `nym-node` (Linux-only).
//!
//! [`FreeTierEnforcement`] is the one public entry point. It owns a single node-owned
//! `nftables` table `inet nym_free_tier` that holds BOTH subsystems - the `tc`
//! rate-limit pool (task 4) and the walled garden (task 5) - plus the shared
//! purchase-endpoint whitelist. One table means one `nft list ruleset` view of every
//! free-tier rule and one atomic `delete table` teardown. The pool and garden are
//! internal, table-scoped command-builders the facade composes; the facade exposes the
//! peer-lifecycle transitions the wiring drives (`admit`, `send_to_garden`, `confine`,
//! `release`, `reconcile`).
//!
//! Membership is native `nft` sets (~O(1), no `ipset` package). Managers emit
//! [`CommandSpec`]s that a [`CommandRunner`] executes - the split keeps command
//! generation unit-testable without root, while the netns harness validates the live
//! behaviour. If `nft` is absent, `setup` fails loudly (that failure is the startup
//! preflight - the caller must surface it, never degrade silently).

use std::net::{Ipv4Addr, Ipv6Addr};

pub use command::{CommandRunner, CommandSpec, SystemCommandRunner};
pub use enforcement::FreeTierEnforcement;
pub use error::EnforcementError;

mod command;
mod enforcement;
mod error;
mod garden;
mod nft;
mod tc;

/// The single node-owned nft table both subsystems share.
pub(crate) const TABLE: &str = "nym_free_tier";
/// Shared whitelist (purchase-endpoint) sets, referenced by both the pool exemption and
/// the garden allow rules - a single source of truth in the datapath.
pub(crate) const ALLOW_V4: &str = "allow_v4";
pub(crate) const ALLOW_V6: &str = "allow_v6";

/// A free peer's dual-stack tunnel addresses (its WireGuard allowed IPs). Both families
/// are always present, so enforcement changes touch v4 and v6 as a pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerAddrs {
    pub v4: Ipv4Addr,
    pub v6: Ipv6Addr,
}
