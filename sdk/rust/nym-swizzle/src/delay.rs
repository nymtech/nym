// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Randomized delay scheduling of async work.
//!
//! A [`Delay`] wraps a future and schedules it after a freshly sampled delay:
//!
//! ```no_run
//! # async fn broadcast_tx() -> u32 { 0 }
//! # async fn example() {
//! use std::time::Duration;
//! use nym_swizzle::Delay;
//!
//! let mut s = Delay::uniform(Duration::ZERO, Duration::from_secs(10));
//! let result = s.run(async move { broadcast_tx().await }).await;
//! # }
//! ```
//!
//! **Guarantee:** the wrapped future is not polled before its scheduled time.
//! Because async blocks are lazy, any side effect inside the block (the
//! broadcast itself) cannot happen early — the observable event *is* the
//! execution, and the execution is what gets delayed.
//!
//! Successive [`Delay::run`] calls on the same instance draw independent
//! samples.

use std::future::Future;
use std::time::Duration;

use crate::rng::{sample_bounded, validate_sampling, CryptoRng, RngSource, Sampling};

/// A configured randomized-delay scheduler. Construct with
/// [`uniform`](Delay::uniform), [`poisson`](Delay::poisson) or
/// [`normal`](Delay::normal), then adjust bounds and randomness source with
/// the builder methods.
#[derive(Debug)]
pub struct Delay {
    min: Duration,
    /// `None` means unbounded above (possible for poisson/normal only).
    max: Option<Duration>,
    /// Sampling configuration in nanosecond space.
    sampling: Sampling,
    rng: RngSource,
}

impl Delay {
    /// Delays sampled uniformly from `[min, max]`.
    ///
    /// Panics if `min > max`.
    pub fn uniform(min: Duration, max: Duration) -> Self {
        assert!(
            min <= max,
            "delay bounds inverted: min {min:?} > max {max:?}"
        );
        Self {
            min,
            max: Some(max),
            sampling: Sampling::Uniform,
            rng: RngSource::default(),
        }
    }

    /// Poisson-process delays: exponential inter-arrival times with the given
    /// mean — the same distribution family the nym mixnet uses for
    /// cover-traffic delays. Unbounded above until [`max`](Delay::max) is set;
    /// out-of-bounds samples are rejection-resampled, never clamped.
    ///
    /// Panics if `mean` is zero.
    pub fn poisson(mean: Duration) -> Self {
        let mean_nanos = mean.as_nanos() as f64;
        let sampling = Sampling::Poisson { mean: mean_nanos };
        validate_sampling(&sampling);
        Self {
            min: Duration::ZERO,
            max: None,
            sampling,
            rng: RngSource::default(),
        }
    }

    /// Normally distributed delays with the given mean and standard
    /// deviation. Negative samples are always rejected (a delay cannot be
    /// negative); restrict further with [`bounds`](Delay::bounds).
    ///
    /// Panics if `std_dev` is zero.
    pub fn normal(mean: Duration, std_dev: Duration) -> Self {
        let sampling = Sampling::Normal {
            mean: mean.as_nanos() as f64,
            std_dev: std_dev.as_nanos() as f64,
        };
        validate_sampling(&sampling);
        Self {
            min: Duration::ZERO,
            max: None,
            sampling,
            rng: RngSource::default(),
        }
    }

    /// Set the minimum delay.
    ///
    /// Panics if a maximum is set and `min` exceeds it.
    pub fn min(mut self, min: Duration) -> Self {
        if let Some(max) = self.max {
            assert!(
                min <= max,
                "delay bounds inverted: min {min:?} > max {max:?}"
            );
        }
        self.min = min;
        self
    }

    /// Set the maximum delay. Out-of-bounds samples from unbounded
    /// distributions are rejection-resampled into `[min, max]`.
    ///
    /// Panics if `max` is below the configured minimum.
    pub fn max(mut self, max: Duration) -> Self {
        assert!(
            self.min <= max,
            "delay bounds inverted: min {:?} > max {max:?}",
            self.min
        );
        self.max = Some(max);
        self
    }

    /// Set both bounds at once. The pair is validated against itself, not
    /// against previously configured bounds, so any valid pair is accepted
    /// regardless of the sampler's current configuration.
    ///
    /// Panics if `min > max`.
    pub fn bounds(mut self, min: Duration, max: Duration) -> Self {
        assert!(
            min <= max,
            "delay bounds inverted: min {min:?} > max {max:?}"
        );
        self.min = min;
        self.max = Some(max);
        self
    }

    /// Use a deterministic ChaCha20 generator seeded with `seed`: the same
    /// seed and configuration reproduce the same delay sequence. Seeds derived
    /// externally (e.g. from a VRF output) are treated as opaque seed
    /// material.
    pub fn seed(mut self, seed: [u8; 32]) -> Self {
        self.rng = RngSource::seeded(seed);
        self
    }

    /// Use a caller-supplied crypto-grade random generator.
    pub fn with_rng<R: CryptoRng + Send + 'static>(mut self, rng: R) -> Self {
        self.rng = RngSource::custom(rng);
        self
    }

    /// Sample one delay from the configured distribution within the
    /// configured bounds.
    pub fn sample(&mut self) -> Duration {
        let min = self.min.as_nanos() as f64;
        let max = self
            .max
            .map(|m| m.as_nanos() as f64)
            .unwrap_or(f64::INFINITY);
        let nanos = sample_bounded(&mut self.rng, &self.sampling, min, max);
        Duration::from_nanos(nanos.round() as u64)
    }

    /// Sample a delay, wait it out, then execute `future` and return its
    /// output. The future is not polled before its scheduled time.
    pub async fn run<F: Future>(&mut self, future: F) -> F::Output {
        let delay = self.sample();
        crate::timer::sleep(delay).await;
        future.await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_samples_within_bounds() {
        let mut d =
            Delay::uniform(Duration::from_millis(5), Duration::from_millis(50)).seed([3u8; 32]);
        for _ in 0..1000 {
            let s = d.sample();
            assert!(s >= Duration::from_millis(5) && s <= Duration::from_millis(50));
        }
    }

    #[test]
    fn poisson_samples_respect_max() {
        let mut d = Delay::poisson(Duration::from_secs(3))
            .max(Duration::from_secs(5))
            .seed([4u8; 32]);
        for _ in 0..1000 {
            assert!(d.sample() <= Duration::from_secs(5));
        }
    }

    #[test]
    fn normal_samples_respect_bounds() {
        let mut d = Delay::normal(Duration::from_secs(5), Duration::from_secs(2))
            .bounds(Duration::from_secs(1), Duration::from_secs(8))
            .seed([5u8; 32]);
        for _ in 0..1000 {
            let s = d.sample();
            assert!(s >= Duration::from_secs(1) && s <= Duration::from_secs(8));
        }
    }

    #[test]
    fn poisson_with_min_only_terminates_and_shifts() {
        // regression: a minimum bound with no maximum previously risked the
        // rejection path; the memoryless shift samples it exactly
        let mut d = Delay::poisson(Duration::from_secs(2))
            .min(Duration::from_secs(600))
            .seed([8u8; 32]);
        let n = 5_000u32;
        let mut total = Duration::ZERO;
        for _ in 0..n {
            let s = d.sample();
            assert!(s >= Duration::from_secs(600));
            total += s;
        }
        let mean = total / n;
        assert!(
            mean >= Duration::from_secs(601) && mean <= Duration::from_secs(603),
            "min-truncated poisson mean should be ~min + mean, got {mean:?}"
        );
    }

    #[test]
    fn bounds_rebind_below_previous_min() {
        // regression: `bounds` used to validate the new max against the old
        // min and panic on this perfectly valid pair
        let mut d = Delay::uniform(Duration::from_secs(100), Duration::from_secs(200))
            .bounds(Duration::ZERO, Duration::from_secs(20))
            .seed([12u8; 32]);
        for _ in 0..100 {
            assert!(d.sample() <= Duration::from_secs(20));
        }
    }

    #[test]
    #[should_panic(expected = "delay bounds inverted")]
    fn bounds_rejects_inverted_pair() {
        let _ = Delay::poisson(Duration::from_secs(1))
            .bounds(Duration::from_secs(9), Duration::from_secs(3));
    }

    #[test]
    fn samples_are_independent_per_call() {
        let mut d = Delay::uniform(Duration::ZERO, Duration::from_secs(1000)).seed([6u8; 32]);
        let first = d.sample();
        // over 100 draws from a 1000s window, a repeat of the first draw on
        // every call would mean the sampler is stuck
        assert!(
            (0..100).map(|_| d.sample()).any(|s| s != first),
            "sampler returned the same value on every call"
        );
    }

    #[test]
    fn same_seed_reproduces_delay_sequence() {
        let mut a = Delay::poisson(Duration::from_secs(2))
            .max(Duration::from_secs(30))
            .seed([9u8; 32]);
        let mut b = Delay::poisson(Duration::from_secs(2))
            .max(Duration::from_secs(30))
            .seed([9u8; 32]);
        for _ in 0..100 {
            assert_eq!(a.sample(), b.sample());
        }
    }

    #[test]
    #[should_panic(expected = "delay bounds inverted")]
    fn inverted_bounds_panic() {
        let _ = Delay::uniform(Duration::from_secs(2), Duration::from_secs(1));
    }
}
