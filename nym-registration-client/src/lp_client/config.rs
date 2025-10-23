// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Configuration for LP (Lewes Protocol) client operations.
//!
//! Provides sane defaults for registration-only protocol. No user configuration needed.

use std::time::Duration;

/// Configuration for LP (Lewes Protocol) connections.
///
/// This configuration is optimized for registration-only LP protocol with sane defaults
/// based on real network conditions and typical registration flow timing.
///
/// # Default Values
/// - `connect_timeout`: 10 seconds - reasonable for real network conditions
/// - `handshake_timeout`: 15 seconds - allows for Noise handshake round-trips
/// - `registration_timeout`: 30 seconds - includes credential verification and response
/// - `tcp_nodelay`: true - lower latency for small registration messages
/// - `tcp_keepalive`: None - not needed for short-lived registration connections
///
/// # Design
/// Since LP is registration-only (connections close after registration completes),
/// these defaults are chosen to:
/// - Fail fast enough for good UX (no indefinite hangs)
/// - Allow sufficient time for real network conditions
/// - Optimize for latency over throughput (small messages)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LpConfig {
    /// TCP connection timeout (nym-102).
    ///
    /// Maximum time to wait for TCP connection establishment.
    /// Default: 10 seconds.
    pub connect_timeout: Duration,

    /// Noise protocol handshake timeout (nym-102).
    ///
    /// Maximum time to wait for Noise handshake completion (all round-trips).
    /// Default: 15 seconds.
    pub handshake_timeout: Duration,

    /// Registration request/response timeout (nym-102).
    ///
    /// Maximum time to wait for registration request send + response receive.
    /// Includes credential verification on gateway side.
    /// Default: 30 seconds.
    pub registration_timeout: Duration,

    /// Enable TCP_NODELAY (disable Nagle's algorithm) (nym-104).
    ///
    /// When true, disables Nagle's algorithm for lower latency.
    /// Recommended for registration messages which are small and latency-sensitive.
    /// Default: true.
    pub tcp_nodelay: bool,

    /// TCP keepalive duration (nym-104).
    ///
    /// When Some, enables TCP keepalive with specified interval.
    /// Since LP is registration-only with short-lived connections, keepalive is not needed.
    /// Default: None.
    pub tcp_keepalive: Option<Duration>,
}

impl Default for LpConfig {
    fn default() -> Self {
        Self {
            // nym-102: Sane timeout defaults for real network conditions
            connect_timeout: Duration::from_secs(10),
            handshake_timeout: Duration::from_secs(15),
            registration_timeout: Duration::from_secs(30),

            // nym-104: Optimized for registration-only protocol
            tcp_nodelay: true,   // Lower latency for small messages
            tcp_keepalive: None, // Not needed for ephemeral connections
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LpConfig::default();

        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.handshake_timeout, Duration::from_secs(15));
        assert_eq!(config.registration_timeout, Duration::from_secs(30));
        assert_eq!(config.tcp_nodelay, true);
        assert_eq!(config.tcp_keepalive, None);
    }

    #[test]
    fn test_config_clone() {
        let config = LpConfig::default();
        let cloned = config.clone();

        assert_eq!(config, cloned);
    }
}
