// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx_types::{delays, Delay};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy)]
#[error("the route vector contains {available} nodes while {requested} hops are required")]
pub struct InvalidNumberOfHops {
    available: usize,
    requested: u8,
}

pub fn generate_hop_delays(average_packet_delay: Duration, num_hops: usize) -> Vec<Delay> {
    if average_packet_delay.is_zero() {
        vec![nym_sphinx_types::Delay::new_from_millis(0); num_hops]
    } else {
        delays::generate_from_average_duration(num_hops, average_packet_delay)
    }
}
