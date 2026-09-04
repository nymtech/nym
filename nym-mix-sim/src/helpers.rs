// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use rand::Rng;
use rand_distr::{Distribution, Exp};

/// Exponential with mean 50 ms.
pub fn generate_mix_delay(rng: &mut impl Rng) -> u64 {
    // SAFETY : hardcoded > 0 value
    #[allow(clippy::unwrap_used)]
    let exp: Exp<f64> = Exp::new(1.0 / 50.0).unwrap();
    exp.sample(rng).round() as u64
}

/// Exponential with mean 20 ms.
pub fn generate_sending_delay(rng: &mut impl Rng) -> Duration {
    // SAFETY : hardcoded > 0 value
    #[allow(clippy::unwrap_used)]
    let exp: Exp<f64> = Exp::new(1.0 / 20.0).unwrap();
    Duration::from_millis(exp.sample(rng).round() as u64)
}

/// Exponential with mean 200 ms.
pub fn generate_cover_traffic_delay(rng: &mut impl Rng) -> Duration {
    // SAFETY : hardcoded > 0 value
    #[allow(clippy::unwrap_used)]
    let exp: Exp<f64> = Exp::new(1.0 / 200.0).unwrap();
    Duration::from_millis(exp.sample(rng).round() as u64)
}
