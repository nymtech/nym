// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx_types::{Delay, delays};
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

#[cfg(test)]
mod tests {
    use super::*;

    // Witness for nym/openspec/mix-node-delay/the-sender-draws-one-exponential-delay-for-each-hop
    #[test]
    fn a_zero_mean_gives_zero_delays() {
        let delays = generate_hop_delays(Duration::ZERO, 4);
        assert_eq!(delays.len(), 4);
        assert!(delays.iter().all(|delay| delay.to_nanos() == 0));
    }

    // Witness for nym/openspec/mix-node-delay/the-sender-draws-one-exponential-delay-for-each-hop
    #[test]
    fn one_delay_for_each_hop() {
        assert_eq!(generate_hop_delays(Duration::from_millis(15), 4).len(), 4);
    }
}
