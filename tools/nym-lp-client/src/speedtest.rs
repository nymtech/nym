//! Speedtest implementation
//!
//! Echo request/reply for RTT measurement.
//! Throughput testing for bandwidth measurement.

#![allow(unused)]

use serde::{Deserialize, Serialize};

/// Speedtest results
#[derive(Debug, Serialize, Deserialize)]
pub struct SpeedtestResult {
    pub gateway: String,
    pub connection: ConnectionResult,
    pub ping: Option<PingResult>,
    pub throughput: Option<ThroughputResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionResult {
    pub success: bool,
    pub handshake_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PingResult {
    pub sent: u32,
    pub received: u32,
    pub min_rtt_ms: f64,
    pub avg_rtt_ms: f64,
    pub max_rtt_ms: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThroughputResult {
    pub bytes: u64,
    pub duration_ms: u64,
    pub kbps: f64,
}

// TODO: Implement speedtest functions
// - run_ping_test() - echo request/reply via SURB
// - run_throughput_test() - bulk data transfer
