// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Live example: fetching Zcash compact blocks from a public lightwalletd
//! over gRPC, naively versus with `nym-swizzle` overlapping chunking.
//!
//! A light client that asks a lightwalletd for exactly the blocks it needs
//! hands the server a precise statement of its interest, and — because each
//! session's start height is the previous session's end — a key that links
//! its sessions together. `nym-swizzle` breaks the request into randomly
//! sized, deliberately overlapping, randomly ordered chunks so no single
//! request describes the client's actual interval.
//!
//! Three runs over the *same* block range, so the numbers are comparable:
//!
//! 1. **direct** — one `GetBlockRange` call for the whole range (what a naive
//!    wallet does).
//! 2. **swizzled, sequential** — overlapping chunks, one at a time. Isolates
//!    the cost of the deliberate redundancy.
//! 3. **swizzled, concurrent** — the same chunks with bounded concurrency.
//!    Shows how much of that cost wall-clock time recovers.
//!
//! Each run is verified to have downloaded every block in the range, and the
//! redundancy is reported as wastage.
//!
//! ```sh
//! cargo run --release -p nym-swizzle-zcash-example
//!
//! # knobs (defaults shown)
//! ZEC_SERVER=https://zec.rocks:443 ZEC_BLOCKS=1000 ZEC_CONCURRENCY=4 ZEC_TRIALS=3 \
//!     cargo run --release -p nym-swizzle-zcash-example
//! ```
//!
//! This talks to a real public server. Keep the block count modest and the
//! concurrency low — the defaults are deliberately gentle.

mod lightwalletd;

use std::collections::BTreeSet;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use nym_swizzle::Range;

use crate::lightwalletd::Lightwalletd;

const DEFAULT_SERVER: &str = "https://zec.rocks:443";
const DEFAULT_BLOCKS: u64 = 1000;
const DEFAULT_CONCURRENCY: usize = 4;
/// Network timings are noisy; compare strategies on a median, not one sample.
const DEFAULT_TRIALS: usize = 3;
/// Stay well behind the tip so the range can't straddle a reorg mid-run.
const TIP_MARGIN: u64 = 1000;

type BoxError = Box<dyn std::error::Error>;

fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// What one strategy achieved over the range.
#[derive(Default)]
struct RunStats {
    /// gRPC calls issued.
    requests: usize,
    /// Blocks streamed back, counting blocks delivered more than once.
    blocks_streamed: usize,
    /// Distinct heights received.
    unique: BTreeSet<u64>,
    /// Shielded transactions seen (duplicates included).
    transactions: usize,
    elapsed: Duration,
}

impl RunStats {
    fn absorb(&mut self, fetched: lightwalletd::FetchedRange) {
        self.requests += 1;
        self.blocks_streamed += fetched.heights.len();
        self.transactions += fetched.transactions;
        self.unique.extend(fetched.heights);
    }

    /// Fraction of streamed blocks that were redundant re-downloads.
    fn wastage(&self) -> f64 {
        if self.blocks_streamed == 0 {
            return 0.0;
        }
        let extra = self.blocks_streamed - self.unique.len();
        extra as f64 / self.unique.len() as f64
    }

    fn report(&self, label: &str, expected: &BTreeSet<u64>) {
        let complete = &self.unique == expected;
        println!("  {label}");
        println!(
            "    {:>7} gRPC request(s), {:>6} blocks streamed, {:>5} shielded txs",
            self.requests, self.blocks_streamed, self.transactions
        );
        println!(
            "    {:>7} unique blocks — complete coverage: {}",
            self.unique.len(),
            if complete {
                "YES".to_string()
            } else {
                let missing = expected.difference(&self.unique).count();
                format!("NO ({missing} missing)")
            }
        );
        assert!(complete, "{label}: did not download the whole range");
    }
}

/// Repeated trials of one strategy. Network timings are noisy enough that a
/// single sample says very little, so every strategy is run several times and
/// compared on the median.
struct Measured {
    stats: RunStats,
    times: Vec<Duration>,
}

impl Measured {
    /// Run `trials` iterations, keeping the first run's coverage statistics
    /// (they are identical across trials — the plan is seeded) and every
    /// run's elapsed time.
    async fn collect<F, Fut>(trials: usize, mut run: F) -> Result<Self, BoxError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<RunStats, BoxError>>,
    {
        let mut times = Vec::with_capacity(trials);
        let mut stats: Option<RunStats> = None;
        for _ in 0..trials {
            let run = run().await?;
            times.push(run.elapsed);
            stats.get_or_insert(run);
        }
        times.sort_unstable();
        Ok(Self {
            stats: stats.expect("at least one trial"),
            times,
        })
    }

    fn median(&self) -> Duration {
        self.times[self.times.len() / 2]
    }

    fn report(&self, label: &str, expected: &BTreeSet<u64>) {
        self.stats.report(label, expected);
        println!(
            "    {:>7.1}% wastage (redundant re-downloads); {:.2}s median over {} trials \
             ({:.2}s–{:.2}s)",
            self.stats.wastage() * 100.0,
            self.median().as_secs_f64(),
            self.times.len(),
            self.times
                .first()
                .expect("at least one trial")
                .as_secs_f64(),
            self.times.last().expect("at least one trial").as_secs_f64(),
        );
    }
}

/// One `GetBlockRange` call for the entire range — the naive wallet.
async fn run_direct(client: &Lightwalletd, start: u64, end: u64) -> Result<RunStats, BoxError> {
    let mut client = client.clone();
    let mut stats = RunStats::default();
    let started = Instant::now();
    stats.absorb(client.block_range(start, end).await?);
    stats.elapsed = started.elapsed();
    Ok(stats)
}

/// Overlapping, randomly ordered chunks, executed with `concurrency` calls in
/// flight (1 = strictly sequential).
///
/// Both swizzled runs are given the same `seed`, so they execute a byte-identical
/// plan and concurrency is the only variable between them.
async fn run_swizzled(
    client: &Lightwalletd,
    start: u64,
    end: u64,
    concurrency: usize,
    seed: [u8; 32],
) -> Result<RunStats, BoxError> {
    let total = end - start;
    let plan = Range::new(start, end)
        .chunk_size((total / 10).max(20)..=(total / 4).max(50))
        .overlap((total / 100).max(2)..=(total / 20).max(10))
        .seed(seed)
        .plan();

    let chunks = plan.len();
    let stats = Mutex::new(RunStats::default());
    let failures = Mutex::new(Vec::new());

    let started = Instant::now();
    plan.for_each_concurrent(concurrency, |chunk_start, chunk_end| {
        let mut client = client.clone();
        let stats = &stats;
        let failures = &failures;
        async move {
            match client.block_range(chunk_start, chunk_end).await {
                Ok(fetched) => stats.lock().expect("stats poisoned").absorb(fetched),
                Err(e) => failures
                    .lock()
                    .expect("failures poisoned")
                    .push(format!("{chunk_start}..{chunk_end}: {e}")),
            }
        }
    })
    .await;

    let failures = failures.into_inner().expect("failures poisoned");
    if !failures.is_empty() {
        return Err(format!(
            "{} of {chunks} chunk(s) failed: {failures:?}",
            failures.len()
        )
        .into());
    }

    let mut stats = stats.into_inner().expect("stats poisoned");
    stats.elapsed = started.elapsed();
    Ok(stats)
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let server: String = env_or("ZEC_SERVER", DEFAULT_SERVER.to_string());
    let blocks: u64 = env_or("ZEC_BLOCKS", DEFAULT_BLOCKS);
    let concurrency: usize = env_or("ZEC_CONCURRENCY", DEFAULT_CONCURRENCY);
    let trials: usize = env_or("ZEC_TRIALS", DEFAULT_TRIALS);
    assert!(blocks > 0, "ZEC_BLOCKS must be positive");
    assert!(trials > 0, "ZEC_TRIALS must be positive");

    println!("connecting to {server} ...");
    let mut client = Lightwalletd::connect(server).await?;

    // the first call also warms the TLS/HTTP2 connection, so the timings
    // below measure block transfer rather than handshake
    let tip = client.tip().await?;
    let end = tip - TIP_MARGIN;
    let start = end - blocks;
    println!("chain tip {tip}; fetching blocks {start}..{end} ({blocks} blocks)\n");

    let expected: BTreeSet<u64> = (start..end).collect();

    // Fetch the range once and discard it, so every timed run below faces an
    // equally warm server-side cache and the ordering of runs doesn't bias
    // the comparison.
    println!("warming the server cache over the range ...\n");
    client.clone().block_range(start, end).await?;

    // One seed for both swizzled runs: identical chunk plan, so the only
    // difference between runs 2 and 3 is concurrency. Derived from the tip so
    // it still varies between invocations.
    let mut seed = [0u8; 32];
    seed[..8].copy_from_slice(&tip.to_le_bytes());

    println!("1. direct fetch (no obfuscation)");
    let direct = Measured::collect(trials, || run_direct(&client, start, end)).await?;
    direct.report("one request for the exact range:", &expected);

    println!("\n2. swizzled fetch, sequential");
    let sequential =
        Measured::collect(trials, || run_swizzled(&client, start, end, 1, seed)).await?;
    sequential.report("overlapping chunks, one at a time:", &expected);

    println!("\n3. swizzled fetch, {concurrency} concurrent");
    let concurrent = Measured::collect(trials, || {
        run_swizzled(&client, start, end, concurrency, seed)
    })
    .await?;
    concurrent.report(
        &format!("overlapping chunks, {concurrency} in flight:"),
        &expected,
    );
    assert_eq!(
        (sequential.stats.requests, sequential.stats.blocks_streamed),
        (concurrent.stats.requests, concurrent.stats.blocks_streamed),
        "the two swizzled runs should have executed an identical plan"
    );

    let baseline = direct.median().as_secs_f64();
    let row = |label: &str, m: &Measured| {
        println!(
            "  {label:<26} {:.2}s   {:>5.2}x baseline, {} request(s), {:.1}% waste",
            m.median().as_secs_f64(),
            m.median().as_secs_f64() / baseline,
            m.stats.requests,
            m.stats.wastage() * 100.0
        );
    };
    println!("\n== summary (median of {trials} trials) ==");
    row("direct", &direct);
    row("swizzled, sequential", &sequential);
    row(&format!("swizzled, {concurrency} concurrent"), &concurrent);
    println!(
        "\nall runs downloaded every block in {start}..{end}; runs 2 and 3 executed a\n\
         byte-identical plan (same seed), so concurrency is the only variable between\n\
         them. Obfuscation costs bandwidth ({:.1}% extra blocks); concurrency is what\n\
         buys the wall-clock back.",
        concurrent.stats.wastage() * 100.0
    );
    println!(
        "\nNote: this hides the *shape* of the request only. A wallet must also avoid\n\
         broadcasting over the sync connection, and should obfuscate its start height\n\
         (nym_swizzle::Range::start_jitter / snap_start) so successive sessions cannot\n\
         be chained by their resume point."
    );

    Ok(())
}
