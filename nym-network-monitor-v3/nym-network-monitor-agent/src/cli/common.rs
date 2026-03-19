// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use crate::agent::config::Config;
use std::net::SocketAddr;
use std::time::Duration;

#[derive(clap::Args, Debug)]
pub(crate) struct CommonArgs {
    /// Specifies for how long the agent should be sending test packets with the specified rate.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "30s", env = NYM_NETWORK_MONITOR_AGENT_SENDING_DURATION_ARG)]
    sending_duration: Duration,

    /// Specifies how long the agent will wait to receive any leftover packets after finishing sending.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "5s", env = NYM_NETWORK_MONITOR_AGENT_WAITING_DURATION_ARG)]
    waiting_duration: Duration,

    /// How long the node itself should delay the packet
    /// It shouldn't be set to zero as otherwise the node will not put the packet through
    /// its delay queue and we would not test the entire pipeline
    #[arg(long, value_parser = humantime::parse_duration, default_value = "50ms", env = NYM_NETWORK_MONITOR_AGENT_PACKET_DELAY_ARG)]
    packet_delay: Duration,

    /// Specifies the target rate of packets (per second) to be sent.
    #[arg(long, default_value = "1000", env = NYM_NETWORK_MONITOR_AGENT_TARGET_RATE_ARG)]
    target_rate: usize,

    /// Specifies whether the agent should reuse the same header for all packets.
    /// And consequently replay them
    #[arg(long, short, default_value = "true", env = NYM_NETWORK_MONITOR_AGENT_REUSE_HEADER_ARG)]
    reuse_header: bool,

    /// Timeout for establishing the TCP connection to the node under test.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "5s", env = NYM_NETWORK_MONITOR_AGENT_EGRESS_CONNECTION_TIMEOUT_ARG)]
    egress_connection_timeout: Duration,

    /// Timeout for completing the Noise handshake with the node under test.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "3s", env = NYM_NETWORK_MONITOR_AGENT_NOISE_HANDSHAKE_TIMEOUT_ARG)]
    noise_handshake_timeout: Duration,

    /// Number of packets sent in a single batch. Together with `target_rate` this controls
    /// how frequently batches are dispatched: one batch every `sending_batch_size / target_rate` seconds.
    #[arg(long, default_value = "50", env = NYM_NETWORK_MONITOR_AGENT_SENDING_BATCH_SIZE_ARG)]
    sending_batch_size: usize,

    /// Specifies the path to the noise key file used for establishing tunnel with the node being tested
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_NOISE_KEY_PATH_ARG)]
    pub(crate) noise_key_path: String,
}

impl CommonArgs {
    /// Constructs a [`Config`] from the common CLI arguments.
    /// `mixnet_address` is provided separately as it is command-specific.
    pub(crate) fn build_config(&self, mixnet_address: SocketAddr) -> Config {
        Config {
            sending_duration: self.sending_duration,
            waiting_duration: self.waiting_duration,
            packet_delay: self.packet_delay,
            egress_connection_timeout: self.egress_connection_timeout,
            noise_handshake_timeout: self.noise_handshake_timeout,
            sending_batch_size: self.sending_batch_size,
            target_rate: self.target_rate,
            reuse_header: self.reuse_header,
            mixnet_address,
        }
    }
}
