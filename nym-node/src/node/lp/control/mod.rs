// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_metrics::{add_histogram_obs, inc, inc_by};

pub mod client_handler;
pub(crate) mod listener;
pub mod node_handler;

// Histogram buckets for LP operation duration (legacy - used by unused forwarding methods)
const LP_DURATION_BUCKETS: &[f64] = &[0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0];

// Histogram buckets for LP connection lifecycle duration
// LP connections can be very short (registration only: ~1s) or very long (dVPN sessions: hours/days)
// Covers full range from seconds to 24 hours
const LP_CONNECTION_DURATION_BUCKETS: &[f64] = &[
    1.0,     // 1 second
    5.0,     // 5 seconds
    10.0,    // 10 seconds
    30.0,    // 30 seconds
    60.0,    // 1 minute
    300.0,   // 5 minutes
    600.0,   // 10 minutes
    1800.0,  // 30 minutes
    3600.0,  // 1 hour
    7200.0,  // 2 hours
    14400.0, // 4 hours
    28800.0, // 8 hours
    43200.0, // 12 hours
    86400.0, // 24 hours
];

/// Connection lifecycle statistics tracking
struct LpConnectionStats {
    /// When the connection started
    start_time: std::time::Instant,
    /// Total bytes received (including protocol framing)
    bytes_received: u64,
    /// Total bytes sent (including protocol framing)
    bytes_sent: u64,
}

impl LpConnectionStats {
    fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            bytes_received: 0,
            bytes_sent: 0,
        }
    }

    fn duration(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    fn record_bytes_received(&mut self, bytes: usize) {
        self.bytes_received += bytes as u64;
    }

    fn record_bytes_sent(&mut self, bytes: usize) {
        self.bytes_sent += bytes as u64;
    }

    /// Emit connection lifecycle metrics for a client connection
    fn emit_lifecycle_client_metrics(&self, graceful: bool) {
        // Track connection duration
        let duration = self.duration().as_secs_f64();
        add_histogram_obs!(
            "lp_client_connection_duration_seconds",
            duration,
            LP_CONNECTION_DURATION_BUCKETS
        );

        // Track bytes transferred
        inc_by!(
            "lp_client_connection_bytes_received_total",
            self.bytes_received as i64
        );
        inc_by!(
            "lp_client_connection_bytes_sent_total",
            self.bytes_sent as i64
        );

        // Track completion type
        if graceful {
            inc!("lp_client_connections_completed_gracefully");
        } else {
            inc!("lp_client_connections_completed_with_error");
        }
    }

    /// Emit connection lifecycle metrics for a node connection
    fn emit_lifecycle_node_metrics(&self, graceful: bool) {
        // Track connection duration
        let duration = self.duration().as_secs_f64();
        add_histogram_obs!(
            "lp_node_connection_duration_seconds",
            duration,
            LP_CONNECTION_DURATION_BUCKETS
        );

        // Track bytes transferred
        inc_by!(
            "lp_node_connection_bytes_received_total",
            self.bytes_received as i64
        );
        inc_by!(
            "lp_node_connection_bytes_sent_total",
            self.bytes_sent as i64
        );

        // Track completion type
        if graceful {
            inc!("lp_node_connections_completed_gracefully");
        } else {
            inc!("lp_node_connections_completed_with_error");
        }
    }
}
