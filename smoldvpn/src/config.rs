// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Datapath configuration types.
//!
//! `smoldvpn` is decoupled from the provisioning facade: a caller obtains a
//! [`PeerConfig`] per hop (e.g. by mapping a `nym-sdk-session` registration) and
//! hands it to the tunnel. The datapath itself needs only raw key material,
//! endpoints and assigned addresses.

use nym_crypto::asymmetric::x25519;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

/// Fixed *client source port* used inside the two-hop inner IPv4/UDP frame — the exit tunnel's
/// packets are framed with this as their UDP source port before being encapsulated to the entry
/// gateway (matching the reference `DEFAULT_EXIT_WG_CLIENT_PORT`, two_hop_config.rs:17). It is NOT a
/// port any node listens on; it only identifies the client side of the inner exit flow.
pub const DEFAULT_EXIT_WG_CLIENT_PORT: u16 = 54001;

/// One WireGuard peer/hop
pub struct PeerConfig {
    /// The gateway's WireGuard public key (x25519).
    pub gateway_public_key: x25519::PublicKey,
    /// The client's WireGuard secret key for this hop (x25519).
    pub client_private_key: x25519::PrivateKey,
    /// LP-negotiated preshared key, if any (a symmetric 32-byte key, not an x25519 key).
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
        // Show non-secret fields (the gateway public key is not secret); never print the client
        // secret key or the preshared key.
        f.debug_struct("PeerConfig")
            .field("gateway_public_key", &self.gateway_public_key.as_bytes())
            .field("client_private_key", &"<redacted>")
            .field("preshared_key", &self.preshared_key.map(|_| "<redacted>"))
            .field("endpoint", &self.endpoint)
            .field("assigned_ipv4", &self.assigned_ipv4)
            .field("assigned_ipv6", &self.assigned_ipv6)
            .finish()
    }
}

/// Per-hop MTU. Defaults follow the reference: overhead 80 B/hop;
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
    /// Mobile targets (iOS/Android) default to the smaller [`MtuConfig::MOBILE`] values; all other
    /// targets default to [`MtuConfig::DESKTOP`].
    fn default() -> Self {
        #[cfg(any(target_os = "ios", target_os = "android"))]
        {
            Self::MOBILE
        }
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        {
            Self::DESKTOP
        }
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
