// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::net::SocketAddr;
use std::time::Duration;

/// Configuration for the [`NetworkMonitorAgent`], controlling packet sending behaviour during a test run.
pub(crate) struct Config {
    /// How long the agent should be sending test packets with the specified rate.
    pub(crate) sending_duration: Duration,

    /// How long the agent will wait to receive any leftover packets after finishing sending.
    pub(crate) waiting_duration: Duration,

    /// How long the target node should delay the packet (i.e. the sphinx delay)
    pub(crate) packet_delay: Duration,

    /// Timeout for establishing the egress connection to the node under test.
    pub(crate) egress_connection_timeout: Duration,

    /// Timeout for the completing the noise handshake.
    pub(crate) noise_handshake_timeout: Duration,

    /// Number of packets sent in a single batch per unit time.
    pub(crate) sending_batch_size: usize,

    /// Target rate of packets (per second) to be sent.
    pub(crate) target_rate: usize,

    /// Whether the agent should reuse the same header for all packets, and consequently replay them.
    pub(crate) reuse_header: bool,

    /// Address of the mixnet listener on this agent
    pub(crate) mixnet_address: SocketAddr,
}

impl Config {
    pub(crate) fn expected_packets(&self) -> usize {
        (self.target_rate as f32 * self.sending_duration.as_secs_f32()).floor() as usize
    }

    pub(crate) fn batch_interval(&self) -> Duration {
        Duration::from_secs_f64(self.sending_batch_size as f64 / self.target_rate as f64)
    }
}
