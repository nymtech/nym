// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Test mode definitions for gateway probe.
//!
//! This module defines the different test modes supported by the gateway probe:
//! - Core: Traditional mixnet path testing and Wireguard via authenticator
//! - WgMix: Wireguard via authenticator
//! - WgLp: Entry LP + Exit LP (nested forwarding) + WireGuard
//! - LpOnly: LP registration only, no WireGuard
//! - Socks5Only: Socks5 test
//! - BridgesOnly: Bridge transports via WG forward path (needs mixnet + credentials)
//! - All: Mixnet, wireguard over authenticator, LP registration and bridge transports
//!
//! Note: Exit policy port checking is handled by the `run-ports` subcommand,
//! not via a test mode.

/// Test mode for the gateway probe.
///
/// Determines which tests are performed and how connections are established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TestMode {
    /// Mixnet tests + WireGuard via authenticator
    #[default]
    Core,
    /// Wireguard via authenticator
    WgMix,
    /// Wireguard over LP
    WgLp,
    /// LP registration only - test handshake and registration
    LpOnly,
    /// Socks5 test only
    Socks5Only,
    /// Bridge transports via WG forward path. Needs mixnet + credentials; not
    /// scored.
    BridgesOnly,
    /// Mixnet tests, Wireguard tests, LP tests, Socks5 test, bridge transports
    All,
}

impl TestMode {
    // Wether we need to run mixnet tests
    pub fn mixnet_tests(&self) -> bool {
        matches!(self, TestMode::Core | TestMode::All)
    }

    // Wether we need to run Wiregurd tests
    pub fn wireguard_tests(&self) -> bool {
        matches!(
            self,
            TestMode::Core | TestMode::WgMix | TestMode::WgLp | TestMode::All
        )
    }

    // Wether we need to run Lp tests
    pub fn lp_tests(&self) -> bool {
        matches!(self, TestMode::WgLp | TestMode::LpOnly | TestMode::All)
    }

    // Wether we need to run socks5 tests
    pub fn socks5_tests(&self) -> bool {
        matches!(self, TestMode::Socks5Only | TestMode::All)
    }

    /// Bridge transports: WG through each published forwarder. Absent from the
    /// directory score.
    pub fn bridge_tests(&self) -> bool {
        matches!(self, TestMode::BridgesOnly | TestMode::All)
    }

    /// Whether this mode requires a mixnet client
    pub fn needs_mixnet(&self) -> bool {
        // Bridges need a WG registration (authenticator over mixnet) so the
        // forward path can be proven, not just the transport dial. That also
        // means credentials are required for `bridges-only`.
        matches!(
            self,
            TestMode::Core | TestMode::All | TestMode::WgMix | TestMode::BridgesOnly
        )
    }
}

impl std::fmt::Display for TestMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestMode::Core => write!(f, "core"),
            TestMode::WgMix => write!(f, "wg-mix"),
            TestMode::WgLp => write!(f, "wg-lp"),
            TestMode::LpOnly => write!(f, "lp-only"),
            TestMode::Socks5Only => write!(f, "socks5-only"),
            TestMode::BridgesOnly => write!(f, "bridges-only"),
            TestMode::All => write!(f, "all"),
        }
    }
}

impl std::str::FromStr for TestMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mixnet" | "core" => Ok(TestMode::Core),
            "wg-mix" | "wgmix" | "wg_mix" => Ok(TestMode::WgMix),
            "wg-lp" | "wglp" | "wg_lp" => Ok(TestMode::WgLp),
            "lp-only" | "lponly" | "lp_only" => Ok(TestMode::LpOnly),
            "socks5-only" | "socks5only" | "socks5_only" => Ok(TestMode::Socks5Only),
            "bridges-only" | "bridgesonly" | "bridges_only" | "bridges" => {
                Ok(TestMode::BridgesOnly)
            }
            "all" => Ok(TestMode::All),
            _ => Err(format!(
                "Unknown test mode: '{s}'. Valid modes: core, wg-mix, wg-lp, lp-only, socks5-only, bridges-only, all"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============ Helper method tests ============

    #[test]
    fn test_needs_mixnet() {
        assert!(TestMode::Core.needs_mixnet());
        assert!(TestMode::WgMix.needs_mixnet());
        assert!(!TestMode::WgLp.needs_mixnet());
        assert!(!TestMode::LpOnly.needs_mixnet());
        assert!(!TestMode::Socks5Only.needs_mixnet());
        assert!(TestMode::BridgesOnly.needs_mixnet());
        assert!(TestMode::All.needs_mixnet());
    }

    // ============ Display tests ============

    #[test]
    fn test_display() {
        assert_eq!(TestMode::Core.to_string(), "core");
        assert_eq!(TestMode::WgMix.to_string(), "wg-mix");
        assert_eq!(TestMode::WgLp.to_string(), "wg-lp");
        assert_eq!(TestMode::LpOnly.to_string(), "lp-only");
        assert_eq!(TestMode::Socks5Only.to_string(), "socks5-only");
        assert_eq!(TestMode::BridgesOnly.to_string(), "bridges-only");
        assert_eq!(TestMode::All.to_string(), "all");
    }

    // ============ FromStr tests ============

    #[test]
    fn test_from_str_canonical() {
        assert_eq!("core".parse::<TestMode>().unwrap(), TestMode::Core);
        assert_eq!("wg-mix".parse::<TestMode>().unwrap(), TestMode::WgMix);
        assert_eq!("wg-lp".parse::<TestMode>().unwrap(), TestMode::WgLp);
        assert_eq!("lp-only".parse::<TestMode>().unwrap(), TestMode::LpOnly);
        assert_eq!(
            "socks5-only".parse::<TestMode>().unwrap(),
            TestMode::Socks5Only
        );
        assert_eq!(
            "bridges-only".parse::<TestMode>().unwrap(),
            TestMode::BridgesOnly
        );
        assert_eq!("all".parse::<TestMode>().unwrap(), TestMode::All);
    }

    #[test]
    fn test_from_str_alternate_formats() {
        // Default aliases
        assert_eq!("mixnet".parse::<TestMode>().unwrap(), TestMode::Core);

        // snake_case
        assert_eq!("wg_mix".parse::<TestMode>().unwrap(), TestMode::WgMix);
        assert_eq!("wg_lp".parse::<TestMode>().unwrap(), TestMode::WgLp);
        assert_eq!("lp_only".parse::<TestMode>().unwrap(), TestMode::LpOnly);
        assert_eq!(
            "socks5_only".parse::<TestMode>().unwrap(),
            TestMode::Socks5Only
        );
        assert_eq!(
            "bridges_only".parse::<TestMode>().unwrap(),
            TestMode::BridgesOnly
        );
        assert_eq!(
            "bridges".parse::<TestMode>().unwrap(),
            TestMode::BridgesOnly
        );

        // no separator
        assert_eq!("wgmix".parse::<TestMode>().unwrap(), TestMode::WgMix);
        assert_eq!("wglp".parse::<TestMode>().unwrap(), TestMode::WgLp);
        assert_eq!("lponly".parse::<TestMode>().unwrap(), TestMode::LpOnly);
        assert_eq!(
            "socks5only".parse::<TestMode>().unwrap(),
            TestMode::Socks5Only
        );
        assert_eq!(
            "bridgesonly".parse::<TestMode>().unwrap(),
            TestMode::BridgesOnly
        );
    }

    #[test]
    fn test_from_str_case_insensitive() {
        assert_eq!("cOrE".parse::<TestMode>().unwrap(), TestMode::Core);
        assert_eq!("WG-MIX".parse::<TestMode>().unwrap(), TestMode::WgMix);
        assert_eq!("Wg_Lp".parse::<TestMode>().unwrap(), TestMode::WgLp);
        assert_eq!("LpOnly".parse::<TestMode>().unwrap(), TestMode::LpOnly);
        assert_eq!(
            "soCkS5-oNlY".parse::<TestMode>().unwrap(),
            TestMode::Socks5Only
        );
        assert_eq!(
            "Bridges-Only".parse::<TestMode>().unwrap(),
            TestMode::BridgesOnly
        );
        assert_eq!("ALL".parse::<TestMode>().unwrap(), TestMode::All);
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
        for mode in [
            TestMode::Core,
            TestMode::WgMix,
            TestMode::WgLp,
            TestMode::LpOnly,
            TestMode::Socks5Only,
            TestMode::BridgesOnly,
            TestMode::All,
        ] {
            let s = mode.to_string();
            let parsed: TestMode = s.parse().unwrap();
            assert_eq!(mode, parsed);
        }
    }
}
