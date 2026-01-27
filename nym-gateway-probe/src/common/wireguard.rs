// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Shared WireGuard tunnel testing via netstack.
//!
//! This module provides common functionality for testing WireGuard tunnels
//! that is shared between different test modes (authenticator-based and LP-based).

use nym_config::defaults::{WG_METADATA_PORT, WG_TUN_DEVICE_IP_ADDRESS_V4};
use tracing::{error, info};

use crate::NetstackArgs;
use crate::netstack::{
    NetstackRequest, NetstackRequestGo, NetstackResult, TwoHopNetstackRequestGo,
};
use crate::types::WgProbeResults;

/// Safe division that returns 0.0 when divisor is 0 (instead of NaN/Inf)
fn safe_ratio(received: u16, sent: u16) -> f32 {
    if sent == 0 {
        0.0
    } else {
        received as f32 / sent as f32
    }
}

/// WireGuard tunnel configuration for netstack testing.
///
/// Contains all the parameters needed to establish and test a WireGuard tunnel.
pub struct WgTunnelConfig {
    /// Client's private IPv4 address in the tunnel
    pub private_ipv4: String,
    /// Client's private IPv6 address in the tunnel
    pub private_ipv6: String,
    /// Client's WireGuard private key (hex encoded)
    pub private_key_hex: String,
    /// Gateway's WireGuard public key (hex encoded)
    pub public_key_hex: String,
    /// WireGuard endpoint address (gateway_ip:port)
    pub endpoint: String,
}

impl WgTunnelConfig {
    /// Create a new tunnel configuration.
    pub fn new(
        private_ipv4: impl Into<String>,
        private_ipv6: impl Into<String>,
        private_key_hex: impl Into<String>,
        public_key_hex: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Self {
        Self {
            private_ipv4: private_ipv4.into(),
            private_ipv6: private_ipv6.into(),
            private_key_hex: private_key_hex.into(),
            public_key_hex: public_key_hex.into(),
            endpoint: endpoint.into(),
        }
    }
}

/// Run WireGuard tunnel connectivity tests using netstack.
///
/// This function tests both IPv4 and IPv6 connectivity through the WireGuard tunnel:
/// - DNS resolution
/// - ICMP ping to specified hosts and IPs
/// - Optional download test
///
/// Results are written directly into the provided `wg_outcome` to avoid field-by-field
/// copying at call sites.
///
/// # Arguments
/// * `config` - WireGuard tunnel configuration
/// * `netstack_args` - Netstack test parameters (DNS, hosts to ping, timeouts, etc.)
/// * `awg_args` - Amnezia WireGuard arguments (empty string for standard WG)
/// * `wg_outcome` - Mutable reference to write test results into
// This function extracts the shared netstack testing logic from
// wg_probe() and wg_probe_lp() to eliminate code duplication.
pub fn run_tunnel_tests(
    config: &WgTunnelConfig,
    netstack_args: &NetstackArgs,
    awg_args: &str,
    wg_outcome: &mut WgProbeResults,
) {
    // Build the netstack request
    let netstack_request = NetstackRequest::new(
        &config.private_ipv4,
        &config.private_ipv6,
        &config.private_key_hex,
        &config.public_key_hex,
        &config.endpoint,
        &format!("http://{WG_TUN_DEVICE_IP_ADDRESS_V4}:{WG_METADATA_PORT}"),
        netstack_args.netstack_download_timeout_sec,
        awg_args,
        netstack_args.clone(),
    );

    // Perform IPv4 ping test
    info!("Testing IPv4 tunnel connectivity...");
    let ipv4_request = NetstackRequestGo::from_rust_v4(&netstack_request);

    match crate::netstack::ping(&ipv4_request) {
        Ok(NetstackResult::Response(netstack_response_v4)) => {
            info!(
                "WireGuard probe response for IPv4: {:#?}",
                netstack_response_v4
            );
            wg_outcome.can_query_metadata_v4 = netstack_response_v4.can_query_metadata;
            wg_outcome.can_handshake_v4 = netstack_response_v4.can_handshake;
            wg_outcome.can_resolve_dns_v4 = netstack_response_v4.can_resolve_dns;
            wg_outcome.ping_hosts_performance_v4 = safe_ratio(
                netstack_response_v4.received_hosts,
                netstack_response_v4.sent_hosts,
            );
            wg_outcome.ping_ips_performance_v4 = safe_ratio(
                netstack_response_v4.received_ips,
                netstack_response_v4.sent_ips,
            );

            wg_outcome.download_duration_sec_v4 = netstack_response_v4.download_duration_sec;
            wg_outcome.download_duration_milliseconds_v4 =
                netstack_response_v4.download_duration_milliseconds;
            wg_outcome.downloaded_file_size_bytes_v4 =
                netstack_response_v4.downloaded_file_size_bytes;
            wg_outcome.downloaded_file_v4 = netstack_response_v4.downloaded_file;
            wg_outcome.download_error_v4 = netstack_response_v4.download_error;
        }
        Ok(NetstackResult::Error { error }) => {
            error!("Netstack runtime error (IPv4): {error}")
        }
        Err(error) => {
            error!("Internal error (IPv4): {error}")
        }
    }

    // Perform IPv6 ping test
    info!("Testing IPv6 tunnel connectivity...");
    let ipv6_request = NetstackRequestGo::from_rust_v6(&netstack_request);

    match crate::netstack::ping(&ipv6_request) {
        Ok(NetstackResult::Response(netstack_response_v6)) => {
            info!(
                "WireGuard probe response for IPv6: {:#?}",
                netstack_response_v6
            );
            wg_outcome.can_handshake_v6 = netstack_response_v6.can_handshake;
            wg_outcome.can_resolve_dns_v6 = netstack_response_v6.can_resolve_dns;
            wg_outcome.ping_hosts_performance_v6 = safe_ratio(
                netstack_response_v6.received_hosts,
                netstack_response_v6.sent_hosts,
            );
            wg_outcome.ping_ips_performance_v6 = safe_ratio(
                netstack_response_v6.received_ips,
                netstack_response_v6.sent_ips,
            );

            wg_outcome.download_duration_sec_v6 = netstack_response_v6.download_duration_sec;
            wg_outcome.download_duration_milliseconds_v6 =
                netstack_response_v6.download_duration_milliseconds;
            wg_outcome.downloaded_file_size_bytes_v6 =
                netstack_response_v6.downloaded_file_size_bytes;
            wg_outcome.downloaded_file_v6 = netstack_response_v6.downloaded_file;
            wg_outcome.download_error_v6 = netstack_response_v6.download_error;
        }
        Ok(NetstackResult::Error { error }) => {
            error!("Netstack runtime error (IPv6): {error}")
        }
        Err(error) => {
            error!("Internal error (IPv6): {error}")
        }
    }
}

/// Two-hop WireGuard tunnel configuration for nested tunnel testing.
///
/// Traffic flows: Exit tunnel -> UDP Forwarder -> Entry tunnel -> Exit Gateway -> Internet
// This is used for LP two-hop mode where traffic must go through entry gateway
// to reach exit gateway. The forwarder bridges the two netstack tunnels on localhost.
pub struct TwoHopWgTunnelConfig {
    // Entry tunnel (outer, connects directly to entry gateway)
    /// Entry client's private IPv4 address in the tunnel
    pub entry_private_ipv4: String,
    /// Entry client's WireGuard private key (hex encoded)
    pub entry_private_key_hex: String,
    /// Entry gateway's WireGuard public key (hex encoded)
    pub entry_public_key_hex: String,
    /// Entry WireGuard endpoint address (entry_gateway_ip:port)
    pub entry_endpoint: String,
    /// Entry Amnezia WG args (empty for standard WG)
    pub entry_awg_args: String,

    // Exit tunnel (inner, connects via forwarder through entry)
    /// Exit client's private IPv4 address in the tunnel
    pub exit_private_ipv4: String,
    /// Exit client's WireGuard private key (hex encoded)
    pub exit_private_key_hex: String,
    /// Exit gateway's WireGuard public key (hex encoded)
    pub exit_public_key_hex: String,
    /// Exit WireGuard endpoint address (exit_gateway_ip:port, forwarded via entry)
    pub exit_endpoint: String,
    /// Exit Amnezia WG args (empty for standard WG)
    pub exit_awg_args: String,
}

impl TwoHopWgTunnelConfig {
    /// Create a new two-hop tunnel configuration.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entry_private_ipv4: impl Into<String>,
        entry_private_key_hex: impl Into<String>,
        entry_public_key_hex: impl Into<String>,
        entry_endpoint: impl Into<String>,
        entry_awg_args: impl Into<String>,
        exit_private_ipv4: impl Into<String>,
        exit_private_key_hex: impl Into<String>,
        exit_public_key_hex: impl Into<String>,
        exit_endpoint: impl Into<String>,
        exit_awg_args: impl Into<String>,
    ) -> Self {
        Self {
            entry_private_ipv4: entry_private_ipv4.into(),
            entry_private_key_hex: entry_private_key_hex.into(),
            entry_public_key_hex: entry_public_key_hex.into(),
            entry_endpoint: entry_endpoint.into(),
            entry_awg_args: entry_awg_args.into(),
            exit_private_ipv4: exit_private_ipv4.into(),
            exit_private_key_hex: exit_private_key_hex.into(),
            exit_public_key_hex: exit_public_key_hex.into(),
            exit_endpoint: exit_endpoint.into(),
            exit_awg_args: exit_awg_args.into(),
        }
    }
}

/// Run two-hop WireGuard tunnel connectivity tests using netstack.
///
/// This function tests connectivity through nested WireGuard tunnels:
/// - Entry tunnel connects directly to entry gateway
/// - UDP forwarder bridges entry and exit tunnels on localhost
/// - Exit tunnel sends traffic via forwarder -> entry tunnel -> exit gateway
/// - Tests (DNS, ping, download) run through the exit tunnel
///
/// # Arguments
/// * `config` - Two-hop WireGuard tunnel configuration
/// * `netstack_args` - Netstack test parameters (DNS, hosts to ping, timeouts, etc.)
/// * `wg_outcome` - Mutable reference to write test results into
// Currently only tests IPv4. IPv6 support can be added later if needed.
pub fn run_two_hop_tunnel_tests(
    config: &TwoHopWgTunnelConfig,
    netstack_args: &NetstackArgs,
    wg_outcome: &mut WgProbeResults,
) {
    // Build the two-hop netstack request for IPv4
    let request = TwoHopNetstackRequestGo {
        // Entry tunnel config
        entry_wg_ip: config.entry_private_ipv4.clone(),
        entry_private_key: config.entry_private_key_hex.clone(),
        entry_public_key: config.entry_public_key_hex.clone(),
        entry_endpoint: config.entry_endpoint.clone(),
        entry_awg_args: config.entry_awg_args.clone(),

        // Exit tunnel config
        exit_wg_ip: config.exit_private_ipv4.clone(),
        exit_private_key: config.exit_private_key_hex.clone(),
        exit_public_key: config.exit_public_key_hex.clone(),
        exit_endpoint: config.exit_endpoint.clone(),
        exit_awg_args: config.exit_awg_args.clone(),

        // Test parameters (use IPv4 config)
        dns: netstack_args.netstack_v4_dns.clone(),
        ip_version: 4,
        ping_hosts: netstack_args.netstack_ping_hosts_v4.clone(),
        ping_ips: netstack_args.netstack_ping_ips_v4.clone(),
        num_ping: netstack_args.netstack_num_ping,
        send_timeout_sec: netstack_args.netstack_send_timeout_sec,
        recv_timeout_sec: netstack_args.netstack_recv_timeout_sec,
        download_timeout_sec: netstack_args.netstack_download_timeout_sec,
    };

    info!("Testing two-hop IPv4 tunnel connectivity...");
    info!("  Entry endpoint: {}", config.entry_endpoint);
    info!("  Exit endpoint (via forwarder): {}", config.exit_endpoint);

    match crate::netstack::ping_two_hop(&request) {
        Ok(NetstackResult::Response(response)) => {
            info!("Two-hop WireGuard probe response (IPv4): {:#?}", response);
            wg_outcome.can_handshake_v4 = response.can_handshake;
            wg_outcome.can_resolve_dns_v4 = response.can_resolve_dns;
            wg_outcome.ping_hosts_performance_v4 =
                safe_ratio(response.received_hosts, response.sent_hosts);
            wg_outcome.ping_ips_performance_v4 =
                safe_ratio(response.received_ips, response.sent_ips);

            wg_outcome.download_duration_sec_v4 = response.download_duration_sec;
            wg_outcome.download_duration_milliseconds_v4 = response.download_duration_milliseconds;
            wg_outcome.downloaded_file_size_bytes_v4 = response.downloaded_file_size_bytes;
            wg_outcome.downloaded_file_v4 = response.downloaded_file;
            wg_outcome.download_error_v4 = response.download_error;
        }
        Ok(NetstackResult::Error { error }) => {
            error!("Two-hop netstack runtime error (IPv4): {error}")
        }
        Err(error) => {
            error!("Two-hop internal error (IPv4): {error}")
        }
    }
}
