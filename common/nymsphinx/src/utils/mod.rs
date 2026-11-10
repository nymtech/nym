// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rand::Rng;
use rand_distr::{Distribution, Exp};
use std::time;

// TODO: ask @AP why we are actually using Distribution::Exp(1/L) rather than just
// Distribution::Poisson(L) directly?

// TODO: should we put an extra trait bound on this to require `CryptoRng`? Could there be any attacks
// because of weak rng used?
// r[impl cover.loop-exponential]
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    // r[verify cover.loop-exponential]
    #[test]
    fn poisson_durations_follow_an_exponential_distribution_with_the_given_mean() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let mean = time::Duration::from_millis(200);
        let samples: Vec<_> = (0..10_000)
            .map(|_| sample_poisson_duration(&mut rng, mean))
            .collect();

        let sample_mean = samples.iter().sum::<time::Duration>() / samples.len() as u32;
        assert!(sample_mean > mean * 95 / 100 && sample_mean < mean * 105 / 100);

        // for an exponential distribution, P(X < mean) = 1 - 1/e, about 0.632. A constant
        // interval or a uniform draw around the mean gives a different fraction.
        let below_mean = samples.iter().filter(|sample| **sample < mean).count();
        let fraction = below_mean as f64 / samples.len() as f64;
        assert!(
            (fraction - 0.632).abs() < 0.03,
            "fraction below mean: {fraction}"
        );
    }
}
