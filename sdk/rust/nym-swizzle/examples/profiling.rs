// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Development-time profiling harness: empirically proves the crate's
//! statistical claims and renders the evidence as SVG plots.
//!
//! Three suites, each pairing plots with hard numeric checks (the plots are
//! evidence; the assertions are the gate — the harness fails loudly, not
//! just prettily):
//!
//! 1. **Delays** — per-distribution histograms overlaid on the theoretical
//!    density restricted to the bounds; sample moments must match theoretical
//!    moments within tolerance, and rejection-resampling must leave no
//!    probability spike at the bounds.
//! 2. **Chunking** — chunk-size / overlap / start-jitter histograms across
//!    many generated plans; every plan must satisfy the coverage invariant.
//! 3. **Seeds** — two identically seeded plans rendered overlaid (and
//!    asserted byte-equal); a differently seeded plan shown diverging.
//!
//! Samples are streamed into fixed-size accumulators (bin counts + running
//! moments), so memory stays flat regardless of sample count.
//!
//! Run with: `cargo run --release -p nym-swizzle --example profiling`
//! Plots land in `<workspace>/target/swizzle-profiling/`.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::Duration;

use plotters::prelude::*;

use nym_swizzle::{Delay, Range, Snap};

const SAMPLES: usize = 10_000_000;
const CHUNK_PLANS: u32 = 50_000;
const JITTER_PLANS: u32 = 500_000;
const BINS: usize = 60;

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = output_dir();
    std::fs::create_dir_all(&out_dir)?;

    delay_suite(&out_dir)?;
    chunk_suite(&out_dir)?;
    seed_suite(&out_dir)?;

    println!("\nall statistical checks passed");
    println!("plots written to {}", out_dir.display());
    Ok(())
}

/// `<workspace>/target/swizzle-profiling`, resolved from the crate manifest.
fn output_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join("target/swizzle-profiling")
}

/// Streaming sample accumulator: fixed-size histogram bins plus running
/// moments, so profiling millions of samples needs O(BINS) memory.
struct Sampled {
    min: f64,
    max: f64,
    bins: Vec<u64>,
    count: u64,
    sum: f64,
    sum_sq: f64,
}

impl Sampled {
    fn new(min: f64, max: f64) -> Self {
        Self {
            min,
            max,
            bins: vec![0; BINS],
            count: 0,
            sum: 0.0,
            sum_sq: 0.0,
        }
    }

    fn record(&mut self, x: f64) {
        assert!(
            x >= self.min && x <= self.max,
            "sample {x} outside bounds [{}, {}]",
            self.min,
            self.max
        );
        let width = (self.max - self.min) / BINS as f64;
        let idx = (((x - self.min) / width) as usize).min(BINS - 1);
        self.bins[idx] += 1;
        self.count += 1;
        self.sum += x;
        self.sum_sq += x * x;
    }

    fn mean(&self) -> f64 {
        self.sum / self.count as f64
    }

    fn std_dev(&self) -> f64 {
        let m = self.mean();
        (self.sum_sq / self.count as f64 - m * m).sqrt()
    }

    /// Per-bin empirical densities at bin centres.
    fn density(&self) -> Vec<(f64, f64)> {
        let width = (self.max - self.min) / BINS as f64;
        self.bins
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                let centre = self.min + (i as f64 + 0.5) * width;
                (centre, c as f64 / (self.count as f64 * width))
            })
            .collect()
    }
}

/// Fail (panic) unless `actual` is within `tolerance` (fractional) of
/// `expected`.
fn check_moment(name: &str, actual: f64, expected: f64, tolerance: f64) {
    let deviation = (actual - expected).abs() / expected.abs().max(f64::EPSILON);
    assert!(
        deviation <= tolerance,
        "{name}: {actual:.4} deviates {:.2}% from expected {expected:.4} (tolerance {:.2}%)",
        deviation * 100.0,
        tolerance * 100.0
    );
    println!(
        "  ok: {name} = {actual:.4} (expected {expected:.4}, within {:.1}%)",
        tolerance * 100.0
    );
}

/// Render an empirical histogram (as a step line) with the theoretical
/// density overlaid.
fn density_plot(
    path: &Path,
    title: &str,
    sampled: &Sampled,
    theoretical: impl Fn(f64) -> f64,
) -> Result<(), Box<dyn Error>> {
    let (min, max) = (sampled.min, sampled.max);
    let empirical = sampled.density();
    let theory: Vec<(f64, f64)> = (0..=400)
        .map(|i| {
            let x = min + (max - min) * i as f64 / 400.0;
            (x, theoretical(x))
        })
        .collect();

    let y_max = empirical
        .iter()
        .map(|&(_, d)| d)
        .chain(theory.iter().map(|&(_, d)| d))
        .fold(0.0f64, f64::max)
        * 1.15;

    let root = SVGBackend::new(path, (900, 500)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 22))
        .margin(12)
        .x_label_area_size(35)
        .y_label_area_size(55)
        .build_cartesian_2d(min..max, 0.0..y_max)?;
    chart
        .configure_mesh()
        .x_desc("value")
        .y_desc("density")
        .draw()?;

    chart
        .draw_series(LineSeries::new(empirical, BLUE.stroke_width(2)))?
        .label("empirical")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE.stroke_width(2)));
    chart
        .draw_series(LineSeries::new(theory, RED.stroke_width(2)))?
        .label("theoretical")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED.stroke_width(2)));
    chart
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.85))
        .draw()?;
    root.present()?;
    println!("  plot: {}", path.display());
    Ok(())
}

// ------------------------------------------------------------------- suite 1

fn delay_suite(out: &Path) -> Result<(), Box<dyn Error>> {
    println!("\n== delay suite: sampled delays follow their configured distributions ==");
    println!("  ({SAMPLES} samples per distribution)");

    // --- uniform over [1s, 5s]
    let mut d = Delay::uniform(Duration::from_secs(1), Duration::from_secs(5)).seed([1; 32]);
    let mut sampled = Sampled::new(1.0, 5.0);
    for _ in 0..SAMPLES {
        sampled.record(d.sample().as_secs_f64());
    }
    check_moment("uniform mean", sampled.mean(), 3.0, 0.005);
    check_moment(
        "uniform std dev",
        sampled.std_dev(),
        4.0 / 12f64.sqrt(),
        0.01,
    );
    density_plot(
        &out.join("delay_uniform.svg"),
        "uniform delay [1s, 5s]: empirical vs theoretical",
        &sampled,
        |_| 1.0 / 4.0,
    )?;

    // --- poisson process (exponential inter-arrival), mean 2s, max 10s
    let lambda = 1.0 / 2.0;
    let max = 10.0;
    let mut d = Delay::poisson(Duration::from_secs(2))
        .max(Duration::from_secs(10))
        .seed([2; 32]);
    let mut sampled = Sampled::new(0.0, max);
    for _ in 0..SAMPLES {
        sampled.record(d.sample().as_secs_f64());
    }
    // truncated exponential: mean = 1/λ − M·e^(−λM) / (1 − e^(−λM))
    let trunc = 1.0 - (-lambda * max).exp();
    let expected_mean = 1.0 / lambda - max * (-lambda * max).exp() / trunc;
    check_moment(
        "poisson (truncated exp) mean",
        sampled.mean(),
        expected_mean,
        0.005,
    );
    // rejection-resampling must not pile mass at the max bound: the density
    // is strictly decreasing, so the last bin must stay below the first
    let bins = sampled.density();
    assert!(
        bins.last().unwrap().1 < bins.first().unwrap().1 / 2.0,
        "unexpected probability mass at the max bound — clamping artefact?"
    );
    println!("  ok: no boundary spike at the max bound");
    density_plot(
        &out.join("delay_poisson.svg"),
        "poisson-process delay (mean 2s, max 10s): empirical vs truncated-exp",
        &sampled,
        move |x| lambda * (-lambda * x).exp() / trunc,
    )?;

    // --- normal, mean 5s, std 1s, bounds [1s, 9s] (±4σ: truncation negligible)
    let mut d = Delay::normal(Duration::from_secs(5), Duration::from_secs(1))
        .bounds(Duration::from_secs(1), Duration::from_secs(9))
        .seed([3; 32]);
    let mut sampled = Sampled::new(1.0, 9.0);
    for _ in 0..SAMPLES {
        sampled.record(d.sample().as_secs_f64());
    }
    check_moment("normal mean", sampled.mean(), 5.0, 0.005);
    check_moment("normal std dev", sampled.std_dev(), 1.0, 0.01);
    density_plot(
        &out.join("delay_normal.svg"),
        "normal delay (mean 5s, std 1s, bounds [1s, 9s]): empirical vs theoretical",
        &sampled,
        |x| (-0.5 * (x - 5.0f64).powi(2)).exp() / (2.0 * std::f64::consts::PI).sqrt(),
    )?;

    Ok(())
}

// ------------------------------------------------------------------- suite 2

fn chunk_suite(out: &Path) -> Result<(), Box<dyn Error>> {
    println!("\n== chunk suite: plan geometry follows its distributions, coverage holds ==");
    println!("  ({CHUNK_PLANS} plans, {JITTER_PLANS} jitter observations)");

    let (range_start, range_end) = (0u64, 100_000u64);
    let (chunk_lo, chunk_hi) = (100u64, 300u64);
    let (ov_lo, ov_hi) = (5u64, 50u64);

    let mut sizes = Sampled::new(chunk_lo as f64, chunk_hi as f64 + 1.0);
    let mut overlaps = Sampled::new(ov_lo as f64, ov_hi as f64 + 1.0);
    for i in 0..CHUNK_PLANS {
        let mut seed = [0u8; 32];
        seed[..4].copy_from_slice(&i.to_le_bytes());
        let plan = Range::new(range_start, range_end)
            .chunk_size(chunk_lo..=chunk_hi)
            .overlap(ov_lo..=ov_hi)
            .seed(seed)
            .plan();

        // coverage invariant on every plan
        let chunks = plan.chunks();
        assert_eq!(chunks.first().unwrap().0, range_start);
        assert_eq!(chunks.last().unwrap().1, range_end);
        for w in chunks.windows(2) {
            assert!(
                w[1].0 > w[0].0 && w[1].0 <= w[0].1,
                "coverage violated: {w:?}"
            );
        }

        for (i, &(s, e)) in chunks.iter().enumerate() {
            if i + 1 < chunks.len() {
                sizes.record((e - s) as f64);
            }
            if i > 0 {
                overlaps.record((chunks[i - 1].1 - s) as f64);
            }
        }
    }
    println!("  ok: coverage invariant held across {CHUNK_PLANS} plans");

    check_moment(
        "chunk size mean",
        sizes.mean(),
        (chunk_lo + chunk_hi) as f64 / 2.0,
        0.005,
    );
    check_moment(
        "overlap mean",
        overlaps.mean(),
        (ov_lo + ov_hi) as f64 / 2.0,
        0.005,
    );
    let size_span = (chunk_hi - chunk_lo + 1) as f64;
    density_plot(
        &out.join("chunk_sizes.svg"),
        "chunk sizes: empirical vs uniform [100, 300]",
        &sizes,
        move |_| 1.0 / size_span,
    )?;
    let ov_span = (ov_hi - ov_lo + 1) as f64;
    density_plot(
        &out.join("chunk_overlaps.svg"),
        "consecutive-chunk overlaps: empirical vs uniform [5, 50]",
        &overlaps,
        move |_| 1.0 / ov_span,
    )?;

    // --- start jitter distribution
    let max_jitter = 1000u64;
    let true_start = 10_000u64;
    let mut jitters = Sampled::new(0.0, max_jitter as f64 + 1.0);
    for i in 0..JITTER_PLANS {
        let mut seed = [0u8; 32];
        seed[..4].copy_from_slice(&i.to_le_bytes());
        seed[4] = 0xaa;
        let plan = Range::new(true_start, 12_000)
            .start_jitter(max_jitter)
            .seed(seed)
            .plan();
        assert!(plan.start() <= true_start);
        jitters.record((true_start - plan.start()) as f64);
    }
    check_moment(
        "start-jitter mean",
        jitters.mean(),
        max_jitter as f64 / 2.0,
        0.005,
    );
    density_plot(
        &out.join("start_jitter.svg"),
        "start-jitter magnitudes: empirical vs uniform [0, 1000]",
        &jitters,
        move |_| 1.0 / (max_jitter as f64 + 1.0),
    )?;

    // --- snapping: many true starts collapse onto few emitted starts
    let spacing = 100u64;
    for true_start in (1000..1100).step_by(7) {
        let plan = Range::new(true_start, 2000)
            .snap_start(Snap::Spacing(spacing))
            .seed([9; 32])
            .plan();
        assert_eq!(
            plan.start(),
            1000,
            "snapping must collapse the whole interval"
        );
    }
    println!("  ok: snapping collapses every start in an interval onto its checkpoint");

    Ok(())
}

// ------------------------------------------------------------------- suite 3

fn seed_suite(out: &Path) -> Result<(), Box<dyn Error>> {
    println!("\n== seed suite: seeded (VRF-style) runs are honoured ==");

    let seed_a = [42u8; 32];
    let mut seed_b = seed_a;
    seed_b[0] ^= 0xff;

    let make = |seed: [u8; 32]| {
        Range::new(0, 2000)
            .chunk_size(50..=150)
            .overlap(5..=20)
            .seed(seed)
            .plan()
    };

    let first = make(seed_a);
    let second = make(seed_a);
    let divergent = make(seed_b);

    assert_eq!(
        first.chunks(),
        second.chunks(),
        "same seed must reproduce the plan"
    );
    println!(
        "  ok: identically seeded plans are byte-identical ({} chunks)",
        first.len()
    );
    assert_ne!(
        first.chunks(),
        divergent.chunks(),
        "different seed must diverge"
    );
    println!("  ok: differently seeded plan diverges");

    // delay sequences reproduce too
    let delays = |seed: [u8; 32]| {
        let mut d = Delay::poisson(Duration::from_secs(2))
            .max(Duration::from_secs(20))
            .seed(seed);
        (0..1000).map(|_| d.sample()).collect::<Vec<_>>()
    };
    assert_eq!(delays(seed_a), delays(seed_a));
    println!("  ok: identically seeded delay sequences are identical");

    // render: plan A and B overlaid (identical), divergent plan offset above
    let path = out.join("seed_determinism.svg");
    let root = SVGBackend::new(&path, (900, 600)).into_drawing_area();
    root.fill(&WHITE)?;
    let rows = first.len().max(divergent.len()) as f64;
    let mut chart = ChartBuilder::on(&root)
        .caption(
            "seeded determinism: runs 1+2 (same seed, overlaid) vs run 3 (different seed)",
            ("sans-serif", 20),
        )
        .margin(12)
        .x_label_area_size(35)
        .y_label_area_size(55)
        .build_cartesian_2d(0.0..2000.0, -1.0..rows)?;
    chart
        .configure_mesh()
        .x_desc("index")
        .y_desc("chunk #")
        .draw()?;

    for (i, &(s, e)) in first.chunks().iter().enumerate() {
        chart.draw_series(LineSeries::new(
            vec![(s as f64, i as f64 + 0.12), (e as f64, i as f64 + 0.12)],
            BLUE.stroke_width(3),
        ))?;
    }
    for (i, &(s, e)) in second.chunks().iter().enumerate() {
        chart.draw_series(LineSeries::new(
            vec![(s as f64, i as f64 - 0.12), (e as f64, i as f64 - 0.12)],
            GREEN.stroke_width(3),
        ))?;
    }
    for (i, &(s, e)) in divergent.chunks().iter().enumerate() {
        chart.draw_series(LineSeries::new(
            vec![(s as f64, i as f64 + 0.38), (e as f64, i as f64 + 0.38)],
            RED.mix(0.6).stroke_width(2),
        ))?;
    }
    root.present()?;
    println!("  plot: {}", path.display());

    Ok(())
}
