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
// This enum replaces the scattered boolean flags (only_wireguard,
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

#[cfg(test)]
mod tests {
    use super::*;

    // ============ from_flags() tests ============

    #[test]
    fn test_from_flags_default_is_mixnet() {
        // All flags false -> Mixnet (default)
        assert_eq!(
            TestMode::from_flags(false, false, false, false),
            TestMode::Mixnet
        );
    }

    #[test]
    fn test_from_flags_only_wireguard_is_mixnet() {
        // only_wireguard still uses mixnet path (WG via authenticator)
        assert_eq!(
            TestMode::from_flags(true, false, false, false),
            TestMode::Mixnet
        );
    }

    #[test]
    fn test_from_flags_only_lp_registration() {
        // only_lp_registration -> LpOnly (takes priority)
        assert_eq!(
            TestMode::from_flags(false, true, false, false),
            TestMode::LpOnly
        );
        // Even with other flags set, only_lp_registration wins
        assert_eq!(
            TestMode::from_flags(true, true, true, true),
            TestMode::LpOnly
        );
    }

    #[test]
    fn test_from_flags_test_lp_wg_single_hop() {
        // test_lp_wg without exit gateway -> SingleHop
        assert_eq!(
            TestMode::from_flags(false, false, true, false),
            TestMode::SingleHop
        );
    }

    #[test]
    fn test_from_flags_test_lp_wg_two_hop() {
        // test_lp_wg with exit gateway -> TwoHop
        assert_eq!(
            TestMode::from_flags(false, false, true, true),
            TestMode::TwoHop
        );
    }

    #[test]
    fn test_from_flags_has_exit_gateway_alone_is_mixnet() {
        // has_exit_gateway alone doesn't change mode
        assert_eq!(
            TestMode::from_flags(false, false, false, true),
            TestMode::Mixnet
        );
    }

    // ============ Helper method tests ============

    #[test]
    fn test_needs_mixnet() {
        assert!(TestMode::Mixnet.needs_mixnet());
        assert!(!TestMode::SingleHop.needs_mixnet());
        assert!(!TestMode::TwoHop.needs_mixnet());
        assert!(!TestMode::LpOnly.needs_mixnet());
    }

    #[test]
    fn test_uses_lp() {
        assert!(!TestMode::Mixnet.uses_lp());
        assert!(TestMode::SingleHop.uses_lp());
        assert!(TestMode::TwoHop.uses_lp());
        assert!(TestMode::LpOnly.uses_lp());
    }

    #[test]
    fn test_tests_wireguard() {
        assert!(TestMode::Mixnet.tests_wireguard());
        assert!(TestMode::SingleHop.tests_wireguard());
        assert!(TestMode::TwoHop.tests_wireguard());
        assert!(!TestMode::LpOnly.tests_wireguard());
    }

    #[test]
    fn test_needs_exit_gateway() {
        assert!(!TestMode::Mixnet.needs_exit_gateway());
        assert!(!TestMode::SingleHop.needs_exit_gateway());
        assert!(TestMode::TwoHop.needs_exit_gateway());
        assert!(!TestMode::LpOnly.needs_exit_gateway());
    }

    // ============ Display tests ============

    #[test]
    fn test_display() {
        assert_eq!(TestMode::Mixnet.to_string(), "mixnet");
        assert_eq!(TestMode::SingleHop.to_string(), "single-hop");
        assert_eq!(TestMode::TwoHop.to_string(), "two-hop");
        assert_eq!(TestMode::LpOnly.to_string(), "lp-only");
    }

    // ============ FromStr tests ============

    #[test]
    fn test_from_str_canonical() {
        assert_eq!("mixnet".parse::<TestMode>().unwrap(), TestMode::Mixnet);
        assert_eq!("single-hop".parse::<TestMode>().unwrap(), TestMode::SingleHop);
        assert_eq!("two-hop".parse::<TestMode>().unwrap(), TestMode::TwoHop);
        assert_eq!("lp-only".parse::<TestMode>().unwrap(), TestMode::LpOnly);
    }

    #[test]
    fn test_from_str_alternate_formats() {
        // snake_case
        assert_eq!("single_hop".parse::<TestMode>().unwrap(), TestMode::SingleHop);
        assert_eq!("two_hop".parse::<TestMode>().unwrap(), TestMode::TwoHop);
        assert_eq!("lp_only".parse::<TestMode>().unwrap(), TestMode::LpOnly);

        // no separator
        assert_eq!("singlehop".parse::<TestMode>().unwrap(), TestMode::SingleHop);
        assert_eq!("twohop".parse::<TestMode>().unwrap(), TestMode::TwoHop);
        assert_eq!("lponly".parse::<TestMode>().unwrap(), TestMode::LpOnly);
    }

    #[test]
    fn test_from_str_case_insensitive() {
        assert_eq!("MIXNET".parse::<TestMode>().unwrap(), TestMode::Mixnet);
        assert_eq!("Single-Hop".parse::<TestMode>().unwrap(), TestMode::SingleHop);
        assert_eq!("TWO_HOP".parse::<TestMode>().unwrap(), TestMode::TwoHop);
        assert_eq!("LpOnly".parse::<TestMode>().unwrap(), TestMode::LpOnly);
    }

    #[test]
    fn test_from_str_invalid() {
        assert!("invalid".parse::<TestMode>().is_err());
        assert!("".parse::<TestMode>().is_err());
        assert!("mix".parse::<TestMode>().is_err());
    }

    // ============ Roundtrip test ============

    #[test]
    fn test_display_fromstr_roundtrip() {
        for mode in [TestMode::Mixnet, TestMode::SingleHop, TestMode::TwoHop, TestMode::LpOnly] {
            let s = mode.to_string();
            let parsed: TestMode = s.parse().unwrap();
            assert_eq!(mode, parsed);
        }
    }
}
