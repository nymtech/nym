// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use nym_crypto::asymmetric::x25519;
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
    packet_delay: std::time::Duration,

    /// Specifies the target rate of packets (per second) to be sent.
    #[arg(long, default_value = "1000", env = NYM_NETWORK_MONITOR_AGENT_TARGET_RATE_ARG)]
    target_rate: usize,

    /// Specifies whether the agent should reuse the same header for all packets.
    /// And consequently replay them
    #[arg(long, short, default_value = "true", env = NYM_NETWORK_MONITOR_AGENT_REUSE_HEADER_ARG)]
    reuse_header: bool,

    /// Specifies the path to the noise key file used for establishing tunnel with the node being tested
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_NOISE_KEY_PATH_ARG)]
    noise_key_path: String,
}

impl CommonArgs {
    pub(crate) fn load_noise_key(&self) -> anyhow::Result<x25519::PrivateKey> {
        Ok(nym_pemstore::load_key(&self.noise_key_path)?)
    }
}
