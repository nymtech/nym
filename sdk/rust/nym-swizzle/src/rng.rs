// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Randomness sources and bounded distribution sampling shared by the
//! [`delay`](crate::delay) and [`range`](crate::range) primitives.
//!
//! Three tiers of randomness are supported:
//!
//! 1. [`OsRng`](rand::rngs::OsRng) — the default: unpredictable,
//!    cryptographically sourced.
//! 2. A fixed 32-byte seed driving a ChaCha20 CSPRNG — deterministic
//!    reproducibility: the same seed and configuration produce byte-identical
//!    chunk plans and delay sequences. Seeds derived externally (e.g. from a
//!    VRF output) are treated as opaque seed material.
//! 3. A caller-supplied generator implementing [`RngCore`] + [`CryptoRng`].
//!
//! Bounded sampling uses **rejection-resampling**: out-of-bounds samples are
//! discarded and redrawn rather than clamped, so no probability spike
//! accumulates at the bounds.

use rand::rngs::OsRng;
use rand::{CryptoRng, Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rand_distr::{Distribution as _, Exp, Normal};

/// Object-safe combination of [`RngCore`] and [`CryptoRng`], so caller-supplied
/// generators can be stored without generic plumbing. Blanket-implemented for
/// every suitable RNG.
pub trait CryptoRngCore: RngCore + CryptoRng + Send {}

impl<T: RngCore + CryptoRng + Send> CryptoRngCore for T {}

/// The randomness source used by a primitive: `OsRng` by default, or any
/// caller-supplied crypto-grade generator (including a seeded ChaCha20 for
/// deterministic reproduction).
pub(crate) enum RngSource {
    Os(OsRng),
    Custom(Box<dyn CryptoRngCore>),
}

impl Default for RngSource {
    fn default() -> Self {
        RngSource::Os(OsRng)
    }
}

impl RngSource {
    pub(crate) fn seeded(seed: [u8; 32]) -> Self {
        RngSource::Custom(Box::new(ChaCha20Rng::from_seed(seed)))
    }

    pub(crate) fn custom<R: CryptoRngCore + 'static>(rng: R) -> Self {
        RngSource::Custom(Box::new(rng))
    }
}

impl std::fmt::Debug for RngSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RngSource::Os(_) => f.write_str("RngSource::Os"),
            RngSource::Custom(_) => f.write_str("RngSource::Custom"),
        }
    }
}

impl RngCore for RngSource {
    fn next_u32(&mut self) -> u32 {
        match self {
            RngSource::Os(rng) => rng.next_u32(),
            RngSource::Custom(rng) => rng.next_u32(),
        }
    }

    fn next_u64(&mut self) -> u64 {
        match self {
            RngSource::Os(rng) => rng.next_u64(),
            RngSource::Custom(rng) => rng.next_u64(),
        }
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        match self {
            RngSource::Os(rng) => rng.fill_bytes(dest),
            RngSource::Custom(rng) => rng.fill_bytes(dest),
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        match self {
            RngSource::Os(rng) => rng.try_fill_bytes(dest),
            RngSource::Custom(rng) => rng.try_fill_bytes(dest),
        }
    }
}

// safety: both variants hold crypto-grade generators
impl CryptoRng for RngSource {}

/// How a bounded quantity (a delay, a start-overlap magnitude) is sampled.
///
/// All variants are sampled subject to `[min, max]` bounds via
/// rejection-resampling (see module docs). Parameters are expressed in the
/// unit of whatever is being sampled: nanoseconds for delays, index counts for
/// range geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sampling {
    /// Uniform over `[min, max]`.
    Uniform,
    /// Poisson-process sampling: exponential inter-arrival times with the
    /// given mean, the same construction the nym mixnet uses for cover-traffic
    /// delays (cf. `sample_poisson_duration` in `common/nymsphinx`). Delays
    /// drawn from the same family as mixnet cover traffic blend into traffic
    /// an observer already sees.
    Poisson {
        /// Mean of the exponential inter-arrival distribution. Must be > 0.
        mean: f64,
    },
    /// Normally distributed with the given mean and standard deviation.
    Normal {
        /// Mean of the distribution.
        mean: f64,
        /// Standard deviation. Must be > 0.
        std_dev: f64,
    },
}

/// After this many out-of-bounds draws, fall back to a uniform in-bounds draw
/// rather than looping forever on a pathological configuration (e.g. a normal
/// whose mass lies almost entirely outside `[min, max]`).
const MAX_REJECTION_RETRIES: usize = 1000;

/// Sample from `sampling` restricted to `[min, max]` by rejection-resampling.
///
/// Panics if the configuration is invalid (`min > max`, non-finite bounds,
/// non-positive `mean`/`std_dev`); validation happens at primitive
/// construction, so this is a defensive backstop.
pub(crate) fn sample_bounded(rng: &mut RngSource, sampling: &Sampling, min: f64, max: f64) -> f64 {
    assert!(
        min <= max && min.is_finite(),
        "invalid sampling bounds: min = {min}, max = {max}"
    );
    if min == max {
        return min;
    }

    match sampling {
        Sampling::Uniform => {
            assert!(max.is_finite(), "uniform sampling requires a finite max");
            rng.gen_range(min..=max)
        }
        Sampling::Poisson { mean } => {
            assert!(*mean > 0.0, "poisson sampling requires mean > 0");
            let exp = Exp::new(1.0 / mean).expect("mean checked positive above");
            reject_into_bounds(rng, min, max, |rng| exp.sample(rng))
        }
        Sampling::Normal { mean, std_dev } => {
            assert!(*std_dev > 0.0, "normal sampling requires std_dev > 0");
            let normal = Normal::new(*mean, *std_dev).expect("std_dev checked positive above");
            reject_into_bounds(rng, min, max, |rng| normal.sample(rng))
        }
    }
}

fn reject_into_bounds(
    rng: &mut RngSource,
    min: f64,
    max: f64,
    mut draw: impl FnMut(&mut RngSource) -> f64,
) -> f64 {
    for _ in 0..MAX_REJECTION_RETRIES {
        let sample = draw(rng);
        if sample >= min && sample <= max {
            return sample;
        }
    }
    // pathological configuration: virtually no mass inside [min, max]. A
    // uniform in-bounds draw terminates at the cost of a (documented)
    // distributional distortion in this edge case only.
    log::debug!(
        "rejection sampling exhausted {MAX_REJECTION_RETRIES} retries for bounds \
         [{min}, {max}]; falling back to a uniform in-bounds draw"
    );
    if max.is_finite() {
        rng.gen_range(min..=max)
    } else {
        min
    }
}

/// Validate construction-time bounds shared by the public builders.
pub(crate) fn validate_sampling(sampling: &Sampling) {
    match sampling {
        Sampling::Uniform => {}
        Sampling::Poisson { mean } => {
            assert!(
                mean.is_finite() && *mean > 0.0,
                "poisson sampling requires a finite mean > 0, got {mean}"
            );
        }
        Sampling::Normal { mean, std_dev } => {
            assert!(mean.is_finite(), "normal sampling requires a finite mean");
            assert!(
                std_dev.is_finite() && *std_dev > 0.0,
                "normal sampling requires a finite std_dev > 0, got {std_dev}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded() -> RngSource {
        RngSource::seeded([7u8; 32])
    }

    #[test]
    fn uniform_stays_in_bounds() {
        let mut rng = seeded();
        for _ in 0..10_000 {
            let s = sample_bounded(&mut rng, &Sampling::Uniform, 5.0, 10.0);
            assert!((5.0..=10.0).contains(&s));
        }
    }

    #[test]
    fn poisson_rejection_stays_in_bounds() {
        let mut rng = seeded();
        for _ in 0..10_000 {
            let s = sample_bounded(&mut rng, &Sampling::Poisson { mean: 3.0 }, 0.0, 4.0);
            assert!((0.0..=4.0).contains(&s));
        }
    }

    #[test]
    fn normal_rejection_stays_in_bounds() {
        let mut rng = seeded();
        let sampling = Sampling::Normal {
            mean: 5.0,
            std_dev: 3.0,
        };
        for _ in 0..10_000 {
            let s = sample_bounded(&mut rng, &sampling, 2.0, 8.0);
            assert!((2.0..=8.0).contains(&s));
        }
    }

    #[test]
    fn pathological_config_terminates_via_fallback() {
        let mut rng = seeded();
        // normal centred 1000 sigmas away from the allowed window: every draw
        // is rejected, the fallback must kick in and stay in bounds
        let sampling = Sampling::Normal {
            mean: 1_000.0,
            std_dev: 1.0,
        };
        let s = sample_bounded(&mut rng, &sampling, 0.0, 1.0);
        assert!((0.0..=1.0).contains(&s));
    }

    #[test]
    fn degenerate_bounds_return_min() {
        let mut rng = seeded();
        assert_eq!(
            sample_bounded(&mut rng, &Sampling::Poisson { mean: 1.0 }, 2.0, 2.0),
            2.0
        );
    }

    #[test]
    fn seeded_sampling_is_deterministic() {
        let mut a = RngSource::seeded([1u8; 32]);
        let mut b = RngSource::seeded([1u8; 32]);
        for _ in 0..100 {
            assert_eq!(
                sample_bounded(&mut a, &Sampling::Poisson { mean: 5.0 }, 0.0, 20.0),
                sample_bounded(&mut b, &Sampling::Poisson { mean: 5.0 }, 0.0, 20.0),
            );
        }
    }
}
