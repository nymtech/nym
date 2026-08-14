// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Sample delays from the Poisson-process distribution.
//!
//! "Poisson" here means exponential inter-arrival times — the same
//! construction the nym mixnet uses for cover-traffic delays
//! (`sample_poisson_duration` in `common/nymsphinx`). Delays drawn from the
//! same family as mixnet cover traffic blend into traffic an observer
//! already sees.
//!
//! The maximum bound is enforced by rejection-resampling (redraw, never
//! clamp), so no probability spike accumulates at the bound.
//!
//! Run with: `cargo run -p nym-swizzle --example poisson_sampling`

use std::time::Duration;

use nym_swizzle::Delay;

fn main() {
    let mean = Duration::from_secs(2);
    let max = Duration::from_secs(10);
    let mut sampler = Delay::poisson(mean).max(max);

    println!("poisson-process delays, mean {mean:?}, max {max:?}:");
    let mut total = Duration::ZERO;
    const N: u32 = 10_000;
    for i in 0..N {
        let sample = sampler.sample();
        total += sample;
        if i < 10 {
            println!("  sample {i}: {sample:?}");
        }
    }

    let empirical_mean = total / N;
    println!("  ...");
    println!("empirical mean over {N} samples: {empirical_mean:?}");
    println!("(below the configured {mean:?} because the [0, {max:?}] bound truncates the tail)");
}
