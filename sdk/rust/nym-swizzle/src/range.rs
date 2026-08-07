// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Overlapping randomized range chunking with start-edge obfuscation.
//!
//! A [`Range`] decomposes a requested index range into chunks with randomly
//! sampled sizes, where consecutive chunks deliberately overlap — e.g.
//! `1..100` might become `1..10, 8..12, 11..21, 19..50, 45..75, 74..100`. The
//! union of the chunks always covers the (start-obfuscated) range exactly and
//! never spills past the end; overlapped indexes are fetched more than once
//! by design (deduplication is the caller's job).
//!
//! Two start-edge obfuscations defeat start-height linking across sync
//! sessions:
//!
//! - [`start_jitter`](Range::start_jitter) — *anonymity by noise*: extend the
//!   start a sampled number of indexes downward, so starts stop being exact
//!   pointers to previous ends.
//! - [`snap_start`](Range::snap_start) — *anonymity by collision*: round the
//!   start down to a checkpoint grid, deterministically and without consuming
//!   randomness, so every client resuming within the same interval emits an
//!   identical start.
//!
//! Execution is pull-style ([`ChunkPlan`] is an iterator over `(start, end)`
//! pairs in randomly permuted order) or push-style, quickcheck-like — hand the
//! plan an async closure and it owns execution:
//!
//! ```no_run
//! # async fn get_block(_s: u64, _e: u64) {}
//! # async fn example() {
//! use nym_swizzle::Range;
//!
//! Range::new(0, 1000)
//!     .plan()
//!     .for_each_concurrent(4, |start, end| get_block(start, end))
//!     .await;
//! # }
//! ```

use std::future::Future;
use std::time::Duration;

use futures::StreamExt;
use rand010::seq::SliceRandom;
use rand010::RngExt as _;

use crate::delay::Delay;
use crate::rng::{sample_bounded, validate_sampling, CryptoRng, RngSource, Sampling};

/// A checkpoint grid for [`Range::snap_start`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Snap {
    /// Evenly spaced checkpoints at every multiple of the given spacing
    /// (absolute indexes: `0, s, 2s, ...`).
    Spacing(u64),
    /// An explicit, not necessarily evenly spaced, list of checkpoint
    /// indexes (e.g. a chain's canonical checkpoints). Sorted on
    /// construction by [`Range::snap_start`].
    Checkpoints(Vec<u64>),
}

impl Snap {
    /// The greatest on-grid point `<= x`, if any. Requires a sorted
    /// checkpoint list (guaranteed by [`Range::snap_start`]).
    fn down(&self, x: u64) -> Option<u64> {
        match self {
            Snap::Spacing(spacing) => Some(x - x % spacing),
            Snap::Checkpoints(cps) => match cps.partition_point(|&cp| cp <= x) {
                0 => None,
                n => Some(cps[n - 1]),
            },
        }
    }

    /// The smallest on-grid point `>= x`, if any.
    fn up(&self, x: u64) -> Option<u64> {
        match self {
            Snap::Spacing(spacing) => x.div_ceil(*spacing).checked_mul(*spacing),
            Snap::Checkpoints(cps) => cps.get(cps.partition_point(|&cp| cp < x)).copied(),
        }
    }
}

#[derive(Debug, Clone)]
struct Jitter {
    sampling: Sampling,
    max: u64,
}

/// Builder for an obfuscated chunk plan over `[start, end)`.
///
/// All knobs default to clamped percentage-of-range derivations documented on
/// each method — sensible starting points, **not** validated anonymity
/// parameters.
#[derive(Debug)]
pub struct Range {
    start: u64,
    end: u64,
    chunk_size: Option<(u64, u64)>,
    overlap: Option<(u64, u64)>,
    jitter: Option<Jitter>,
    floor: u64,
    snap: Option<Snap>,
    rng: RngSource,
}

impl Range {
    /// A plan over the half-open index range `[start, end)`.
    ///
    /// If you want to mask *which* sub-range you actually care about, widen
    /// `[start, end)` yourself — the crate obfuscates coverage of whatever
    /// range it is given and never extends the end (it cannot know which
    /// indexes exist).
    ///
    /// Panics if `start >= end`.
    pub fn new(start: u64, end: u64) -> Self {
        assert!(start < end, "empty or inverted range: {start}..{end}");
        Self {
            start,
            end,
            chunk_size: None,
            overlap: None,
            jitter: None,
            floor: 0,
            snap: None,
            rng: RngSource::default(),
        }
    }

    /// Bounds for sampled chunk sizes (inclusive). Defaults to
    /// `[total/50, total/10]`, clamped to at least 1.
    ///
    /// Panics if `min` is zero or `min > max`.
    pub fn chunk_size(mut self, bounds: std::ops::RangeInclusive<u64>) -> Self {
        let (min, max) = bounds.into_inner();
        assert!(min >= 1, "chunk size must be at least 1");
        assert!(min <= max, "chunk size bounds inverted: {min} > {max}");
        self.chunk_size = Some((min, max));
        self
    }

    /// Bounds for the sampled overlap between consecutive chunks (inclusive).
    /// Defaults to `[1, clamp(total/20, 1, max_chunk/2)]`. Overlaps are
    /// clamped down when a sampled chunk is too small to support them (small
    /// ranges stay valid).
    ///
    /// Panics if `min > max`.
    pub fn overlap(mut self, bounds: std::ops::RangeInclusive<u64>) -> Self {
        let (min, max) = bounds.into_inner();
        assert!(min <= max, "overlap bounds inverted: {min} > {max}");
        self.overlap = Some((min, max));
        self
    }

    /// Enable start-edge obfuscation by noise: move the plan's start downward
    /// by an amount sampled uniformly from `[0, max_jitter]` (the end is
    /// never extended). Re-fetching data already held is the point: starts
    /// stop being exact pointers to previous ends.
    pub fn start_jitter(mut self, max_jitter: u64) -> Self {
        self.jitter = Some(Jitter {
            sampling: Sampling::Uniform,
            max: max_jitter,
        });
        self
    }

    /// Like [`start_jitter`](Range::start_jitter) but sampling the jitter
    /// magnitude from an arbitrary distribution (in index units),
    /// rejection-resampled into `[0, max_jitter]`.
    pub fn start_jitter_sampled(mut self, sampling: Sampling, max_jitter: u64) -> Self {
        validate_sampling(&sampling);
        self.jitter = Some(Jitter {
            sampling,
            max: max_jitter,
        });
        self
    }

    /// Enable start jitter with the default magnitude: 10% of the total
    /// range, clamped to at least 1.
    pub fn start_jitter_default(mut self) -> Self {
        let total = self.end - self.start;
        self.jitter = Some(Jitter {
            sampling: Sampling::Uniform,
            max: (total / 10).max(1),
        });
        self
    }

    /// The lowest index the obfuscated start may reach (e.g. a wallet birthday
    /// would leak — prefer an activation height or 0). Defaults to 0.
    pub fn floor(mut self, floor: u64) -> Self {
        self.floor = floor;
        self
    }

    /// Enable start-edge obfuscation by collision: deterministically round
    /// the plan's start *down* to the given checkpoint grid. Consumes no
    /// randomness — every client resuming within the same checkpoint interval
    /// emits an identical start.
    ///
    /// When combined with jitter, the jitter is applied first and snapping
    /// last: the emitted start is always on-grid and jitter moves it down
    /// only in whole checkpoints, so noise widens *which* checkpoint is
    /// chosen without destroying the collision property.
    ///
    /// Checkpoint lists are sorted internally. Panics if given
    /// `Snap::Spacing(0)` or an empty checkpoint list.
    pub fn snap_start(mut self, mut snap: Snap) -> Self {
        match &mut snap {
            Snap::Spacing(spacing) => assert!(*spacing > 0, "snap spacing must be positive"),
            Snap::Checkpoints(cps) => {
                assert!(!cps.is_empty(), "checkpoint list must not be empty");
                cps.sort_unstable();
            }
        }
        self.snap = Some(snap);
        self
    }

    /// Use a deterministic ChaCha20 generator seeded with `seed`: the same
    /// seed and configuration produce a byte-identical plan (chunks and
    /// execution order). Seeds derived externally (e.g. from a VRF output)
    /// are treated as opaque seed material.
    pub fn seed(mut self, seed: [u8; 32]) -> Self {
        self.rng = RngSource::seeded(seed);
        self
    }

    /// Use a caller-supplied crypto-grade random generator.
    pub fn with_rng<R: CryptoRng + Send + 'static>(mut self, rng: R) -> Self {
        self.rng = RngSource::custom(rng);
        self
    }

    /// Materialise the chunk plan: obfuscate the start, decompose into
    /// overlapping chunks, and fix a random execution order.
    pub fn plan(mut self) -> ChunkPlan {
        let total = self.end - self.start;

        // resolved defaults: clamped percentage-of-range derivations
        let (chunk_min, chunk_max) = self
            .chunk_size
            .unwrap_or(((total / 50).max(1), (total / 10).max(1)));
        let (overlap_min, overlap_max) = self
            .overlap
            .unwrap_or((1, (total / 20).clamp(1, (chunk_max / 2).max(1))));

        // one jitter draw regardless of snapping, so enabling snapping never
        // changes the RNG sample stream
        let jitter_amount = self
            .jitter
            .as_ref()
            .map(|j| sample_bounded(&mut self.rng, &j.sampling, 0.0, j.max as f64).round() as u64);

        let start = obfuscated_start(self.start, self.floor, jitter_amount, self.snap.as_ref());

        // decompose [start, end) into overlapping chunks: each chunk begins
        // before its predecessor ends, so the union is gapless by construction
        let mut chunks = Vec::new();
        let mut cursor = start;
        loop {
            let size = self.rng.random_range(chunk_min..=chunk_max);
            let chunk_end = cursor.saturating_add(size).min(self.end);
            chunks.push((cursor, chunk_end));
            if chunk_end == self.end {
                break;
            }
            // overlap cannot consume the whole chunk (progress) nor reach
            // back before the plan start
            let size = chunk_end - cursor;
            let max_valid = (size - 1).min(chunk_end - start);
            let hi = overlap_max.min(max_valid);
            let lo = overlap_min.min(hi);
            let overlap = self.rng.random_range(lo..=hi);
            cursor = chunk_end - overlap;
        }

        let mut order: Vec<usize> = (0..chunks.len()).collect();
        order.shuffle(&mut self.rng);

        ChunkPlan {
            chunks,
            order,
            position: 0,
            delay: None,
        }
    }
}

/// Compute the emitted (obfuscated) start from the true start.
///
/// Pure with respect to randomness: the jitter amount is sampled by the
/// caller, and snapping never consumes randomness.
fn obfuscated_start(true_start: u64, floor: u64, jitter: Option<u64>, snap: Option<&Snap>) -> u64 {
    // jitter moves the start down only, never below the floor (a floor
    // misconfigured above the start binds at the start itself)
    let floor = floor.min(true_start);
    let jittered = true_start.saturating_sub(jitter.unwrap_or(0)).max(floor);

    // snapping is applied last, so with a grid the emitted start is always
    // on-grid and jitter moves it down only in whole checkpoints
    let Some(snap) = snap else { return jittered };
    match snap.down(jittered) {
        Some(snapped) if snapped >= floor => snapped,
        // the grid point below the jittered start is under the floor (or the
        // start is below every checkpoint): take the smallest on-grid point
        // still inside [floor, true_start], or give up on snapping
        _ => snap
            .up(floor)
            .filter(|&up| up <= true_start)
            .unwrap_or(jittered),
    }
}

/// A materialised, shuffled chunk plan produced by [`Range::plan`].
///
/// Iterate it pull-style (`Iterator<Item = (u64, u64)>`, random order), or
/// hand it an async closure with [`for_each`](ChunkPlan::for_each) /
/// [`for_each_concurrent`](ChunkPlan::for_each_concurrent).
#[derive(Debug)]
pub struct ChunkPlan {
    /// Chunks in generation (index) order.
    chunks: Vec<(u64, u64)>,
    /// Random execution permutation over `chunks`.
    order: Vec<usize>,
    /// Pull-iteration cursor into `order`.
    position: usize,
    /// Optional inter-chunk delay for the push drivers.
    delay: Option<Delay>,
}

impl ChunkPlan {
    /// Number of chunks in the plan.
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Whether the plan has no chunks (never true for a valid range).
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// The chunks in generation (ascending index) order — mainly useful for
    /// inspection and testing; execution should use the shuffled iterator or
    /// the push drivers.
    pub fn chunks(&self) -> &[(u64, u64)] {
        &self.chunks
    }

    /// The emitted (possibly obfuscated) start of the plan.
    pub fn start(&self) -> u64 {
        self.chunks.first().map(|c| c.0).unwrap_or(0)
    }

    /// The end of the plan (always the requested end).
    pub fn end(&self) -> u64 {
        self.chunks.last().map(|c| c.1).unwrap_or(0)
    }

    /// Attach an inter-chunk delay used by the push drivers: each chunk's
    /// execution is preceded by a freshly sampled delay.
    pub fn delay(mut self, delay: Delay) -> Self {
        self.delay = Some(delay);
        self
    }

    /// Execute `f` for every chunk sequentially, in the plan's random order,
    /// applying the configured inter-chunk delay (if any) before each chunk.
    pub async fn for_each<F, Fut>(mut self, mut f: F)
    where
        F: FnMut(u64, u64) -> Fut,
        Fut: Future<Output = ()>,
    {
        let mut delay = self.delay.take();
        for (start, end) in &mut self {
            match delay.as_mut() {
                Some(delay) => delay.run(f(start, end)).await,
                None => f(start, end).await,
            }
        }
    }

    /// Execute `f` for up to `limit` chunks concurrently, in the plan's
    /// random order, until every chunk has been executed. When an inter-chunk
    /// delay is configured, each chunk's delay is pre-sampled and elapses
    /// inside its own task, so the schedule stays randomized under
    /// concurrency.
    pub async fn for_each_concurrent<F, Fut>(mut self, limit: usize, f: F)
    where
        F: Fn(u64, u64) -> Fut,
        Fut: Future<Output = ()>,
    {
        let mut delay = self.delay.take();
        let jobs: Vec<((u64, u64), Option<Duration>)> = (&mut self)
            .map(|chunk| (chunk, delay.as_mut().map(|d| d.sample())))
            .collect();

        futures::stream::iter(jobs)
            // lower clamp: `limit.max(1)` raises a zero limit to 1 (futures'
            // concurrency adapters panic on 0); larger limits pass through
            .for_each_concurrent(limit.max(1), |((start, end), delay)| {
                let f = &f;
                async move {
                    if let Some(delay) = delay {
                        crate::timer::sleep(delay).await;
                    }
                    f(start, end).await;
                }
            })
            .await;
    }

    /// Like [`for_each_concurrent`](ChunkPlan::for_each_concurrent), but for
    /// closures that return a value: yields each chunk's output as that chunk
    /// completes (completion order, not plan order), so callers can collect,
    /// fold, or short-circuit without threading results through shared state.
    ///
    /// Exactly one output is yielded per chunk. When an inter-chunk delay is
    /// configured, each chunk's delay is pre-sampled and elapses inside its
    /// own task before the closure's future is polled, exactly as in
    /// [`for_each_concurrent`](ChunkPlan::for_each_concurrent).
    pub fn stream_concurrent<F, Fut, T>(
        mut self,
        limit: usize,
        f: F,
    ) -> impl futures::Stream<Item = T>
    where
        F: Fn(u64, u64) -> Fut,
        Fut: Future<Output = T>,
    {
        let mut delay = self.delay.take();
        let jobs: Vec<((u64, u64), Option<Duration>)> = (&mut self)
            .map(|chunk| (chunk, delay.as_mut().map(|d| d.sample())))
            .collect();

        futures::stream::iter(jobs)
            .map(move |((start, end), delay)| {
                let work = f(start, end);
                async move {
                    if let Some(delay) = delay {
                        crate::timer::sleep(delay).await;
                    }
                    work.await
                }
            })
            // lower clamp: `limit.max(1)` raises a zero limit to 1 (futures'
            // concurrency adapters panic on 0); larger limits pass through
            .buffer_unordered(limit.max(1))
    }
}

impl Iterator for ChunkPlan {
    type Item = (u64, u64);

    fn next(&mut self) -> Option<Self::Item> {
        let idx = *self.order.get(self.position)?;
        self.position += 1;
        Some(self.chunks[idx])
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.order.len() - self.position;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for ChunkPlan {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapping_rounds_down_to_spacing() {
        assert_eq!(
            obfuscated_start(1042, 0, None, Some(&Snap::Spacing(100))),
            1000
        );
        assert_eq!(
            obfuscated_start(1097, 0, None, Some(&Snap::Spacing(100))),
            1000
        );
        assert_eq!(
            obfuscated_start(1100, 0, None, Some(&Snap::Spacing(100))),
            1100
        );
    }

    #[test]
    fn snapping_uses_greatest_checkpoint_not_exceeding_start() {
        // sorted, as Range::snap_start guarantees
        let cps = Snap::Checkpoints(vec![500, 1000, 1500]);
        assert_eq!(obfuscated_start(1042, 0, None, Some(&cps)), 1000);
        assert_eq!(obfuscated_start(1600, 0, None, Some(&cps)), 1500);
        // below every checkpoint: start unchanged
        assert_eq!(obfuscated_start(400, 0, None, Some(&cps)), 400);
    }

    #[test]
    fn snapped_floor_conflict_takes_smallest_grid_point_in_range() {
        // jitter pushes to the floor (950); the grid point below (900) is
        // under the floor, so the smallest on-grid point in [950, 1042] wins
        assert_eq!(
            obfuscated_start(1042, 950, Some(500), Some(&Snap::Spacing(100))),
            1000
        );
    }

    #[test]
    fn jitter_with_spacing_stays_on_grid() {
        for jitter in [0u64, 50, 99, 100, 250, 1000] {
            let s = obfuscated_start(1042, 0, Some(jitter), Some(&Snap::Spacing(100)));
            assert_eq!(s % 100, 0, "off-grid start {s} for jitter {jitter}");
            assert!(s <= 1000);
        }
    }

    #[test]
    fn jitter_without_snap_respects_floor() {
        assert_eq!(obfuscated_start(1000, 950, Some(200), None), 950);
        assert_eq!(obfuscated_start(1000, 0, Some(200), None), 800);
        assert_eq!(obfuscated_start(1000, 0, Some(0), None), 1000);
    }
}
