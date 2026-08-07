// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Randomness sources and bounded distribution sampling shared by the
//! [`delay`](crate::delay) and [`range`](crate::range) primitives.
//!
//! Three tiers of randomness are supported:
//!
//! 1. The operating system's entropy source ([`getrandom04::SysRng`]) — the
//!    default: unpredictable, cryptographically sourced.
//! 2. A fixed 32-byte seed driving a ChaCha20 CSPRNG — deterministic
//!    reproducibility: the same seed and configuration produce byte-identical
//!    chunk plans and delay sequences. Seeds derived externally (e.g. from a
//!    VRF output) are treated as opaque seed material.
//! 3. A caller-supplied generator implementing [`CryptoRng`] (which in
//!    `rand_core` ≥ 0.9 already implies the base RNG interface).
//!
//! Bounded sampling uses **rejection-resampling**: out-of-bounds samples are
//! discarded and redrawn rather than clamped, so no probability spike
//! accumulates at the bounds.

use std::convert::Infallible;

use getrandom04::SysRng;
use rand010::rand_core::UnwrapErr;
use rand010::{RngExt as _, SeedableRng, TryCryptoRng, TryRng};
use rand_chacha010::ChaCha20Rng;
use rand_distr06::{Distribution as _, Exp, Normal};

/// The marker trait caller-supplied generators must implement, re-exported
/// from `rand_core` (via `rand` 0.10): `CryptoRng` is a subtrait of the base
/// RNG interface, so a single bound covers both.
pub use rand010::CryptoRng;

/// The randomness source used by a primitive: OS entropy by default, or any
/// caller-supplied crypto-grade generator (including a seeded ChaCha20 for
/// deterministic reproduction).
pub(crate) enum RngSource {
    Os(UnwrapErr<SysRng>),
    Custom(Box<dyn CryptoRng + Send>),
}

impl Default for RngSource {
    fn default() -> Self {
        RngSource::Os(UnwrapErr(SysRng))
    }
}

impl RngSource {
    pub(crate) fn seeded(seed: [u8; 32]) -> Self {
        RngSource::Custom(Box::new(ChaCha20Rng::from_seed(seed)))
    }

    pub(crate) fn custom<R: CryptoRng + Send + 'static>(rng: R) -> Self {
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

// The rand_core 0.10 idiom: implement `TryRng` with an infallible error and
// mark `TryCryptoRng`; blanket impls then provide `Rng` and `CryptoRng`.
impl TryRng for RngSource {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        match self {
            RngSource::Os(rng) => rng.try_next_u32(),
            RngSource::Custom(rng) => rng.try_next_u32(),
        }
    }

    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        match self {
            RngSource::Os(rng) => rng.try_next_u64(),
            RngSource::Custom(rng) => rng.try_next_u64(),
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Infallible> {
        match self {
            RngSource::Os(rng) => rng.try_fill_bytes(dest),
            RngSource::Custom(rng) => rng.try_fill_bytes(dest),
        }
    }
}

// safety: both variants hold crypto-grade generators
impl TryCryptoRng for RngSource {}

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

/// After this many out-of-bounds draws, use the distribution-appropriate
/// termination fallback rather than looping forever on a pathological
/// configuration (e.g. a normal whose mass lies almost entirely outside
/// `[min, max]`). The fallback is never a constant — a point mass at a bound
/// is the fingerprint spike this module exists to prevent.
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
            rng.random_range(min..=max)
        }
        Sampling::Poisson { mean } => {
            assert!(*mean > 0.0, "poisson sampling requires mean > 0");
            let exp = Exp::new(1.0 / mean).expect("mean checked positive above");
            if max.is_infinite() {
                // exact, not a fallback: by memorylessness, the exponential
                // conditioned on exceeding `min` is exactly `min` plus a
                // fresh exponential draw — no rejection needed, and no
                // distortion however far `min` sits into the tail
                return min + exp.sample(rng);
            }
            reject_into_bounds(
                rng,
                min,
                max,
                |rng| exp.sample(rng),
                |rng| rng.random_range(min..=max),
            )
        }
        Sampling::Normal { mean, std_dev } => {
            assert!(*std_dev > 0.0, "normal sampling requires std_dev > 0");
            let normal = Normal::new(*mean, *std_dev).expect("std_dev checked positive above");
            reject_into_bounds(
                rng,
                min,
                max,
                |rng| normal.sample(rng),
                |rng| {
                    if max.is_finite() {
                        rng.random_range(min..=max)
                    } else {
                        // no memoryless trick exists for the normal: fall back to
                        // a shifted half-normal — still a documented distortion
                        // in this pathological case, but continuous, with no
                        // point mass at `min` (a constant would be a fingerprint
                        // spike, the artifact rejection-resampling exists to
                        // prevent)
                        let half =
                            Normal::new(0.0, *std_dev).expect("std_dev checked positive above");
                        min + half.sample(rng).abs()
                    }
                },
            )
        }
    }
}

fn reject_into_bounds(
    rng: &mut RngSource,
    min: f64,
    max: f64,
    mut draw: impl FnMut(&mut RngSource) -> f64,
    fallback: impl FnOnce(&mut RngSource) -> f64,
) -> f64 {
    for _ in 0..MAX_REJECTION_RETRIES {
        let sample = draw(rng);
        if sample >= min && sample <= max {
            return sample;
        }
    }
    // pathological configuration: virtually no mass inside [min, max]. The
    // distribution-appropriate fallback terminates at the cost of a
    // (documented) distributional distortion in this edge case only.
    tracing::debug!(
        "rejection sampling exhausted {MAX_REJECTION_RETRIES} retries for bounds \
         [{min}, {max}]; using the termination fallback"
    );
    fallback(rng)
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
    fn min_only_exponential_is_memoryless_shift() {
        let mut rng = seeded();
        // min sits 60 means into the tail: naive rejection would exhaust
        // every retry budget; the memoryless shift is exact and immediate
        let sampling = Sampling::Poisson { mean: 1.0 };
        let n = 20_000;
        let mut sum = 0.0;
        let mut at_min = 0usize;
        for _ in 0..n {
            let s = sample_bounded(&mut rng, &sampling, 60.0, f64::INFINITY);
            assert!(s >= 60.0);
            sum += s;
            if s == 60.0 {
                at_min += 1;
            }
        }
        let mean = sum / n as f64;
        assert!(
            (60.9..61.1).contains(&mean),
            "min-truncated exponential mean should be min + mean, got {mean}"
        );
        assert!(at_min == 0, "continuous sampler put point mass at min");
    }

    #[test]
    fn min_only_normal_fallback_is_not_constant() {
        let mut rng = seeded();
        // mass entirely below min with no upper bound: retries exhaust, and
        // the shifted half-normal fallback must not collapse to a constant
        let sampling = Sampling::Normal {
            mean: -1_000.0,
            std_dev: 1.0,
        };
        let draws: Vec<f64> = (0..50)
            .map(|_| sample_bounded(&mut rng, &sampling, 5.0, f64::INFINITY))
            .collect();
        assert!(draws.iter().all(|&s| s >= 5.0));
        assert!(
            draws.iter().any(|&s| s != draws[0]),
            "unbounded-max fallback returned a constant — a boundary spike"
        );
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
