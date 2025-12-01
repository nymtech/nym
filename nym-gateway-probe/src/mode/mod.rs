// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Test mode definitions for gateway probe.
//!
//! This module defines the different test modes supported by the gateway probe:
//! - Mixnet: Traditional mixnet path testing
//! - SingleHop: LP registration + WireGuard on single gateway
//! - TwoHop: Entry LP + Exit LP (nested forwarding) + WireGuard
//! - LpOnly: LP registration only, no WireGuard

/// Test mode for the gateway probe.
///
/// Determines which tests are performed and how connections are established.
// AIDEV-NOTE: This enum replaces the scattered boolean flags (only_wireguard,
// only_lp_registration, test_lp_wg) with explicit, named modes for clarity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TestMode {
    /// Traditional mixnet testing - connects via mixnet, tests entry/exit pings + WireGuard via authenticator
    #[default]
    Mixnet,
    /// LP registration + WireGuard on single gateway (no mixnet, no forwarding)
    SingleHop,
    /// Entry LP + Exit LP (nested session forwarding) + WireGuard tunnel
    TwoHop,
    /// LP registration only - test handshake and registration, skip WireGuard
    LpOnly,
}

impl TestMode {
    /// Infer test mode from legacy boolean flags (backward compatibility)
    pub fn from_flags(
        only_wireguard: bool,
        only_lp_registration: bool,
        test_lp_wg: bool,
        has_exit_gateway: bool,
    ) -> Self {
        if only_lp_registration {
            TestMode::LpOnly
        } else if test_lp_wg {
            if has_exit_gateway {
                TestMode::TwoHop
            } else {
                TestMode::SingleHop
            }
        } else if only_wireguard {
            // WireGuard via authenticator (still uses mixnet path)
            TestMode::Mixnet
        } else {
            TestMode::Mixnet
        }
    }

    /// Whether this mode requires a mixnet client
    pub fn needs_mixnet(&self) -> bool {
        matches!(self, TestMode::Mixnet)
    }

    /// Whether this mode uses LP registration
    pub fn uses_lp(&self) -> bool {
        matches!(self, TestMode::SingleHop | TestMode::TwoHop | TestMode::LpOnly)
    }

    /// Whether this mode tests WireGuard tunnels
    pub fn tests_wireguard(&self) -> bool {
        matches!(self, TestMode::Mixnet | TestMode::SingleHop | TestMode::TwoHop)
    }

    /// Whether this mode requires an exit gateway
    pub fn needs_exit_gateway(&self) -> bool {
        matches!(self, TestMode::TwoHop)
    }
}

impl std::fmt::Display for TestMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestMode::Mixnet => write!(f, "mixnet"),
            TestMode::SingleHop => write!(f, "single-hop"),
            TestMode::TwoHop => write!(f, "two-hop"),
            TestMode::LpOnly => write!(f, "lp-only"),
        }
    }
}

impl std::str::FromStr for TestMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mixnet" => Ok(TestMode::Mixnet),
            "single-hop" | "singlehop" | "single_hop" => Ok(TestMode::SingleHop),
            "two-hop" | "twohop" | "two_hop" => Ok(TestMode::TwoHop),
            "lp-only" | "lponly" | "lp_only" => Ok(TestMode::LpOnly),
            _ => Err(format!(
                "Unknown test mode: '{}'. Valid modes: mixnet, single-hop, two-hop, lp-only",
                s
            )),
        }
    }
}
