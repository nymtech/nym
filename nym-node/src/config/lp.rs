// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_config::serde_helpers::de_maybe_port;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::time::Duration;

/// Configuration for LP listener
#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct LpConfig {
    /// Bind address for the TCP LP control traffic.
    /// default: `[::]:41264`
    pub control_bind_address: SocketAddr,

    /// Bind address for the UDP LP data traffic.
    /// default: `[::]:51264`
    pub data_bind_address: SocketAddr,

    /// Custom announced port for listening for the TCP LP control traffic.
    /// If unspecified, the value from the `control_bind_address` will be used instead
    /// (default: None)
    #[serde(deserialize_with = "de_maybe_port")]
    pub announce_control_port: Option<u16>,

    /// Custom announced port for listening for the UDP LP data traffic.
    /// If unspecified, the value from the `data_bind_address` will be used instead
    /// (default: None)
    #[serde(deserialize_with = "de_maybe_port")]
    pub announce_data_port: Option<u16>,

    /// Auxiliary configuration
    #[serde(default)]
    pub debug: LpDebug,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct LpDebug {
    /// Maximum concurrent connections
    pub max_connections: usize,

    /// Use mock ecash manager for testing (default: false)
    ///
    /// When enabled, the LP listener will use a mock ecash verifier that
    /// accepts any credential without blockchain verification. This is
    /// useful for testing the LP protocol implementation without requiring
    /// a full blockchain/contract setup.
    ///
    /// WARNING: Only use this for local testing! Never enable in production.
    pub use_mock_ecash: bool,

    /// Maximum age of in-progress handshakes before cleanup (default: 90s)
    ///
    /// Handshakes should complete quickly (3-5 packets). This TTL accounts for:
    /// - Network latency and retransmits
    /// - Slow clients
    /// - Clock skew tolerance
    ///
    /// Stale handshakes are removed by the cleanup task to prevent memory leaks.
    #[serde(with = "humantime_serde")]
    pub handshake_ttl: Duration,

    /// Maximum age of established sessions before cleanup (default: 24h)
    ///
    /// Sessions can be long-lived for dVPN tunnels. This TTL should be set
    /// high enough to accommodate expected usage patterns:
    /// - dVPN sessions: hours to days
    /// - Registration: minutes
    ///
    /// Sessions with no activity for this duration are removed by the cleanup task.
    #[serde(with = "humantime_serde")]
    pub session_ttl: Duration,

    /// How often to run the state cleanup task (default: 5 minutes)
    ///
    /// The cleanup task scans for and removes stale handshakes and sessions.
    /// Lower values = more frequent cleanup but higher overhead.
    /// Higher values = less overhead but slower memory reclamation.
    #[serde(with = "humantime_serde")]
    pub state_cleanup_interval: Duration,

    /// Maximum concurrent forward connections (default: 1000)
    ///
    /// Limits simultaneous outbound connections when forwarding LP packets to other gateways
    /// during telescope setup. This prevents file descriptor exhaustion under high load.
    ///
    /// When at capacity, new forward requests return an error, signaling the client
    /// to choose a different gateway.
    pub max_concurrent_forwards: usize,

    /// Number of worker threads processing the LP data plane pipeline.
    ///
    /// Heavy per-packet work (sphinx/outfox decryption, replay-filter check,
    /// fragmentation) is fanned out across this pool. Higher values improve
    /// throughput on multi-core hosts at the cost of more contention on the
    /// shared replay-protection mutex.
    pub data_worker_count: usize,
}

impl LpConfig {
    pub const DEFAULT_CONTROL_PORT: u16 = 41264;
    pub const DEFAULT_DATA_PORT: u16 = 51264;

    pub fn announced_control_port(&self) -> u16 {
        self.announce_control_port
            .unwrap_or(self.control_bind_address.port())
    }

    pub fn announced_data_port(&self) -> u16 {
        self.announce_data_port
            .unwrap_or(self.data_bind_address.port())
    }
}

impl Default for LpConfig {
    fn default() -> Self {
        LpConfig {
            control_bind_address: SocketAddr::new(
                IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                Self::DEFAULT_CONTROL_PORT,
            ),
            data_bind_address: SocketAddr::new(
                IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                Self::DEFAULT_DATA_PORT,
            ),
            announce_control_port: None,
            announce_data_port: None,
            debug: Default::default(),
        }
    }
}

impl LpDebug {
    pub const DEFAULT_MAX_CONNECTIONS: usize = 10000;

    // 90 seconds - handshakes should complete quickly
    pub const DEFAULT_HANDSHAKE_TTL: Duration = Duration::from_secs(90);

    // 24 hours - for long-lived dVPN sessions
    pub const DEFAULT_SESSION_TTL: Duration = Duration::from_secs(86400);

    // 5 minutes - balances memory reclamation with task overhead
    pub const DEFAULT_STATE_CLEANUP_INTERVAL: Duration = Duration::from_secs(300);

    // Limits concurrent outbound connections to prevent fd exhaustion
    pub const DEFAULT_MAX_CONCURRENT_FORWARDS: usize = 1000;

    // Default number of CPU-bound packet-processing workers.
    pub const DEFAULT_DATA_WORKER_COUNT: usize = 4;
}

impl Default for LpDebug {
    fn default() -> Self {
        LpDebug {
            max_connections: Self::DEFAULT_MAX_CONNECTIONS,
            use_mock_ecash: false,
            handshake_ttl: Self::DEFAULT_HANDSHAKE_TTL,
            session_ttl: Self::DEFAULT_SESSION_TTL,
            state_cleanup_interval: Self::DEFAULT_STATE_CLEANUP_INTERVAL,
            max_concurrent_forwards: Self::DEFAULT_MAX_CONCURRENT_FORWARDS,
            data_worker_count: Self::DEFAULT_DATA_WORKER_COUNT,
        }
    }
}
