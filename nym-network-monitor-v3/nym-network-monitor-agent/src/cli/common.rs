// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::time::Duration;

#[derive(clap::Args, Debug)]
pub(crate) struct CommonArgs {
    /// Specifies for how long the agent should be sending test packets with the specified rate.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "30s")]
    sending_duration: Duration,

    /// Specifies how long the agent will wait to receive any leftover packets after finishing sending.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "5s")]
    waiting_duration: Duration,

    /// Specifies the target rate of packets (per second) to be sent.
    #[arg(long, default_value = "1000")]
    target_rate: usize,

    /// Specifies whether the agent should reuse the same header for all packets.
    /// And consequently replay them
    #[arg(long, short, default_value = "true")]
    reuse_header: bool,
}
