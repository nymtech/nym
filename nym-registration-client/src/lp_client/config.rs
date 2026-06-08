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
/// - `forward_timeout`: 30 seconds - forward packet send/receive to exit gateway
/// - `tcp_nodelay`: true - lower latency for small registration messages
/// - `tcp_keepalive`: None - not needed for short-lived registration connections
///
/// # Design
/// Since LP is registration-only (connections close after registration completes),
/// these defaults are chosen to:
/// - Fail fast enough for good UX (no indefinite hangs)
/// - Allow sufficient time for real network conditions
/// - Optimize for latency over throughput (small messages)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LpRegistrationConfig {
    /// TCP connection timeout.
    ///
    /// Maximum time to wait for TCP connection establishment.
    /// Default: 5 seconds.
    pub connect_timeout: Duration,

    /// KKT/PSQ protocol handshake timeout.
    ///
    /// Maximum time to wait for KKT/PSQ handshake completion with the entry (all round-trips).
    /// Default: 8 seconds.
    pub handshake_timeout: Duration,

    /// Registration request/response timeout.
    ///
    /// Maximum time to wait for registration request send + response receive.
    /// Includes credential verification on gateway side.
    /// Default: 8 seconds.
    pub registration_timeout: Duration,

    /// Maximum time for the whole exchange (handshake + registration).
    /// Default: 20 seconds.
    pub exchange_timeout: Duration,

    /// Forward packet send/receive timeout.
    ///
    /// Maximum time to wait for forward packet to get sent via entry gateway.
    /// Default: 3 seconds.
    pub forward_timeout: Duration,

    /// Enable TCP_NODELAY (disable Nagle's algorithm).
    ///
    /// When true, disables Nagle's algorithm for lower latency.
    /// Recommended for registration messages which are small and latency-sensitive.
    /// Default: true.
    pub tcp_nodelay: bool,

    /// TCP keepalive duration.
    ///
    /// When Some, enables TCP keepalive with specified interval.
    /// Since LP is registration-only with short-lived connections, keepalive is not needed.
    /// Default: None.
    pub tcp_keepalive: Option<Duration>,
}

impl Default for LpRegistrationConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            handshake_timeout: Duration::from_secs(8),
            registration_timeout: Duration::from_secs(8),
            exchange_timeout: Duration::from_secs(20),
            forward_timeout: Duration::from_secs(3),

            tcp_nodelay: true,
            tcp_keepalive: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LpRegistrationConfig::default();

        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.handshake_timeout, Duration::from_secs(8));
        assert_eq!(config.registration_timeout, Duration::from_secs(8));
        assert_eq!(config.forward_timeout, Duration::from_secs(3));
        assert_eq!(config.exchange_timeout, Duration::from_secs(20));
        assert!(config.tcp_nodelay);
        assert_eq!(config.tcp_keepalive, None);
    }
}
