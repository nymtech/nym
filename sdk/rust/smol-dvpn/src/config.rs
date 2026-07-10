// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! Datapath configuration types.
//!
//! `smol-dvpn` is decoupled from the provisioning facade: a caller obtains a
//! [`PeerConfig`] per hop (e.g. by mapping a `nym-sdk-session` registration) and
//! hands it to the tunnel. The datapath itself needs only raw key material,
//! endpoints and assigned addresses.

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

/// Fixed exit client source port — the reference `DEFAULT_EXIT_WG_CLIENT_PORT`
/// (two_hop_config.rs:17). Used as a fallback when a dynamic port is not bound.
pub const DEFAULT_EXIT_WG_CLIENT_PORT: u16 = 54001;

/// One WireGuard peer/hop. All key material is raw 32-byte x25519, keeping the
/// datapath independent of any particular crypto wrapper type.
#[derive(Clone)]
pub struct PeerConfig {
    /// The gateway's WireGuard public key (x25519).
    pub gateway_public_key: [u8; 32],
    /// The client's WireGuard private key for this hop (x25519).
    pub client_private_key: [u8; 32],
    /// LP-negotiated preshared key, if any.
    pub preshared_key: Option<[u8; 32]>,
    /// The gateway's WireGuard UDP endpoint.
    pub endpoint: SocketAddr,
    /// Tunnel IPv4 address assigned to the client for this hop.
    pub assigned_ipv4: Ipv4Addr,
    /// Tunnel IPv6 address assigned to the client for this hop, if any.
    pub assigned_ipv6: Option<Ipv6Addr>,
}

impl std::fmt::Debug for PeerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print key material.
        f.debug_struct("PeerConfig")
            .field("endpoint", &self.endpoint)
            .field("assigned_ipv4", &self.assigned_ipv4)
            .field("assigned_ipv6", &self.assigned_ipv6)
            .field("preshared_key", &self.preshared_key.map(|_| "<redacted>"))
            .finish()
    }
}

/// Per-hop MTU. Defaults follow the reference (design D9): overhead 80 B/hop;
/// desktop entry 1420 / exit 1340; mobile entry 1360 / exit 1280.
#[derive(Clone, Copy, Debug)]
pub struct MtuConfig {
    /// Entry-hop MTU.
    pub entry: usize,
    /// Exit-hop MTU (the application-visible interface MTU).
    pub exit: usize,
}

impl MtuConfig {
    /// WireGuard per-hop overhead in bytes (IPv6 worst case).
    pub const OVERHEAD_PER_HOP: usize = 80;
    /// Desktop defaults.
    pub const DESKTOP: MtuConfig = MtuConfig {
        entry: 1420,
        exit: 1340,
    };
    /// Mobile defaults.
    pub const MOBILE: MtuConfig = MtuConfig {
        entry: 1360,
        exit: 1280,
    };
}

impl Default for MtuConfig {
    fn default() -> Self {
        Self::DESKTOP
    }
}

/// DNS behaviour inside the tunnel.
#[derive(Clone, Copy, Debug, Default)]
pub enum DnsMode {
    /// Resolve through the tunnel using the default upstream server.
    #[default]
    InTunnel,
    /// Resolve through the tunnel using a specific upstream server.
    InTunnelServer(SocketAddr),
    /// Do not provide in-tunnel resolution.
    Disabled,
}

/// Tunnel-wide options.
#[derive(Clone, Debug)]
pub struct TunnelConfig {
    /// Per-hop MTU.
    pub mtu: MtuConfig,
    /// DNS behaviour.
    pub dns: DnsMode,
    /// Source port the exit tunnel uses inside the two-hop inner frame.
    pub exit_client_port: u16,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            mtu: MtuConfig::default(),
            dns: DnsMode::default(),
            exit_client_port: DEFAULT_EXIT_WG_CLIENT_PORT,
        }
    }
}
