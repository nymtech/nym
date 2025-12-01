// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Shared WireGuard tunnel testing via netstack.
//!
//! This module provides common functionality for testing WireGuard tunnels
//! that is shared between different test modes (authenticator-based and LP-based).

use nym_config::defaults::{WG_METADATA_PORT, WG_TUN_DEVICE_IP_ADDRESS_V4};
use tracing::{error, info};

use crate::netstack::{NetstackRequest, NetstackRequestGo, NetstackResult};
use crate::types::WgProbeResults;
use crate::NetstackArgs;

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
// AIDEV-NOTE: This function extracts the shared netstack testing logic from
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
