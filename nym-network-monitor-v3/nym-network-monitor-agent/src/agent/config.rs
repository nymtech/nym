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

    /// Number of packets dispatched in a single batch. Together with `target_rate` this
    /// determines the inter-batch interval: `sending_batch_size / target_rate` seconds.
    pub(crate) sending_batch_size: usize,

    /// Target rate of packets (per second) to be sent.
    pub(crate) target_rate: usize,

    /// Whether the agent should reuse the same header for all packets, and consequently replay them.
    pub(crate) reuse_header: bool,

    /// Local socket address the agent binds its mixnet listener on to receive returning packets.
    pub(crate) mixnet_address: SocketAddr,
}

impl Config {
    /// Total number of packets the agent intends to send: `floor(target_rate * sending_duration)`.
    pub(crate) fn expected_packets(&self) -> usize {
        (self.target_rate as f32 * self.sending_duration.as_secs_f32()).floor() as usize
    }

    /// Time between consecutive batch dispatches needed to sustain `target_rate`:
    /// `sending_batch_size / target_rate` seconds.
    pub(crate) fn batch_interval(&self) -> Duration {
        Duration::from_secs_f64(self.sending_batch_size as f64 / self.target_rate as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use std::time::Duration;

    fn config(target_rate: usize, sending_duration: Duration, batch_size: usize) -> Config {
        Config {
            sending_duration,
            waiting_duration: Duration::from_secs(5),
            packet_delay: Duration::from_millis(50),
            egress_connection_timeout: Duration::from_secs(5),
            noise_handshake_timeout: Duration::from_secs(3),
            sending_batch_size: batch_size,
            target_rate,
            reuse_header: true,
            mixnet_address: "127.0.0.1:1789".parse().unwrap(),
        }
    }

    #[test]
    fn expected_packets_floors_fractional_result() {
        // 1000 * 0.5s = 500.0 — exact, no rounding needed
        assert_eq!(
            config(1000, Duration::from_millis(500), 50).expected_packets(),
            500
        );
    }

    #[test]
    fn expected_packets_floors_not_rounds() {
        // 1000 * 1.9s = 1900.0 exactly
        assert_eq!(
            config(1000, Duration::from_millis(1900), 50).expected_packets(),
            1900
        );
    }

    #[test]
    fn batch_interval_is_batch_size_over_rate() {
        // 100 packets / 1000 pps = 100ms
        let interval = config(1000, Duration::from_secs(30), 100).batch_interval();
        assert_eq!(interval, Duration::from_millis(100));
    }

    #[test]
    fn batch_interval_smaller_than_one_ms() {
        // 1 packet / 1000 pps = 1ms
        let interval = config(1000, Duration::from_secs(30), 1).batch_interval();
        assert_eq!(interval, Duration::from_millis(1));
    }
}
