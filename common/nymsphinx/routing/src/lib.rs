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

// r[impl delay.per-hop-exponential]
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

    // r[verify delay.per-hop-exponential]
    #[test]
    fn a_zero_mean_gives_zero_delays() {
        let delays = generate_hop_delays(Duration::ZERO, 4);
        assert_eq!(delays.len(), 4);
        assert!(delays.iter().all(|delay| delay.to_nanos() == 0));
    }

    // r[verify delay.per-hop-exponential]
    #[test]
    fn one_delay_for_each_hop() {
        assert_eq!(generate_hop_delays(Duration::from_millis(15), 4).len(), 4);
    }

    // r[verify delay.per-hop-exponential]
    #[test]
    fn delays_are_fresh_samples_with_the_configured_mean() {
        let mean = Duration::from_millis(15);
        // no generator goes in, so two calls cannot repeat each other's samples
        assert_ne!(
            generate_hop_delays(mean, 4)
                .iter()
                .map(Delay::to_nanos)
                .collect::<Vec<_>>(),
            generate_hop_delays(mean, 4)
                .iter()
                .map(Delay::to_nanos)
                .collect::<Vec<_>>()
        );

        // an exponential sample has a standard deviation equal to its mean, so the mean of
        // 10,000 samples is within 5 % of 15 ms with a margin of 5 standard errors
        let samples = generate_hop_delays(mean, 10_000);
        let sample_mean = samples.iter().map(Delay::to_nanos).sum::<u64>() / 10_000;
        let expected = mean.as_nanos() as u64;
        assert!(
            sample_mean.abs_diff(expected) < expected / 20,
            "{sample_mean} ns"
        );
    }
}
