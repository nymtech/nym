// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rand::Rng;
use rand_distr::{Distribution, Exp};
use std::time;

// TODO: ask @AP why we are actually using Distribution::Exp(1/L) rather than just
// Distribution::Poisson(L) directly?

// TODO: should we put an extra trait bound on this to require `CryptoRng`? Could there be any attacks
// because of weak rng used?
pub fn sample_poisson_duration<R: Rng + ?Sized>(
    rng: &mut R,
    average_duration: time::Duration,
) -> time::Duration {
    // this is our internal code used by our traffic streams
    // the error is only thrown if average delay is less than 0, which will never happen
    // so call to unwrap is perfectly safe here
    let exp = Exp::new(1.0 / average_duration.as_nanos() as f64).unwrap();
    time::Duration::from_nanos(exp.sample(rng).round() as u64)
}
