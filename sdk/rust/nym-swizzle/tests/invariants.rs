// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the spec-level invariants of `nym-swizzle`:
//! coverage, overlap, permutation, start obfuscation, determinism, the delay
//! laziness guarantee, and the push drivers.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use nym_swizzle::{ChunkPlan, Delay, Range, Snap};

fn seed(n: u8) -> [u8; 32] {
    [n; 32]
}

/// Assert the core plan invariants: exact coverage of `[start, end)`, no
/// spill, gapless overlap between generation-order neighbours.
fn assert_plan_invariants(plan: &ChunkPlan, expected_start: u64, end: u64) {
    let chunks = plan.chunks();
    assert!(!chunks.is_empty());
    assert_eq!(
        chunks.first().unwrap().0,
        expected_start,
        "wrong plan start"
    );
    assert_eq!(chunks.last().unwrap().1, end, "wrong plan end");
    for (i, &(s, e)) in chunks.iter().enumerate() {
        assert!(s < e, "empty chunk {s}..{e}");
        assert!(e <= end, "chunk {s}..{e} spills past end {end}");
        if i > 0 {
            let (prev_s, prev_e) = chunks[i - 1];
            assert!(s > prev_s, "no forward progress: {prev_s} -> {s}");
            // gapless: each chunk starts no later than its predecessor ends.
            // (strict overlap degrades to touching only when chunks are too
            // small to support it — the documented small-range clamp)
            assert!(s <= prev_e, "gap between {prev_s}..{prev_e} and {s}..{e}");
        }
    }
}

// ---------------------------------------------------------------- range plans

#[test]
fn plan_covers_range_exactly_across_many_seeds() {
    for n in 0..100u8 {
        let plan = Range::new(0, 1000).seed(seed(n)).plan();
        assert_plan_invariants(&plan, 0, 1000);
    }
}

#[test]
fn plan_respects_configured_chunk_and_overlap_bounds() {
    for n in 0..50u8 {
        let plan = Range::new(0, 10_000)
            .chunk_size(20..=100)
            .overlap(2..=10)
            .seed(seed(n))
            .plan();
        assert_plan_invariants(&plan, 0, 10_000);
        let chunks = plan.chunks();
        for (i, &(s, e)) in chunks.iter().enumerate() {
            // every chunk except the last (clamped to end) honours size bounds
            if i + 1 < chunks.len() {
                assert!(
                    (20..=100).contains(&(e - s)),
                    "chunk size {} out of bounds",
                    e - s
                );
            }
            if i > 0 {
                let overlap = chunks[i - 1].1 - s;
                assert!(
                    (2..=10).contains(&overlap),
                    "overlap {overlap} outside configured [2, 10]"
                );
            }
        }
    }
}

#[test]
fn small_ranges_still_produce_valid_plans() {
    for end in 1..=20u64 {
        for n in 0..10u8 {
            let plan = Range::new(0, end).seed(seed(n)).plan();
            assert_plan_invariants(&plan, 0, end);
        }
    }
}

#[test]
fn offset_ranges_are_covered() {
    let plan = Range::new(4_120_000, 4_121_000).seed(seed(1)).plan();
    assert_plan_invariants(&plan, 4_120_000, 4_121_000);
}

// ------------------------------------------------------------- pull iteration

#[test]
fn iterator_yields_every_chunk_exactly_once_in_permuted_order() {
    let plan = Range::new(0, 5000).seed(seed(42)).plan();
    let generation_order = plan.chunks().to_vec();
    let yielded: Vec<_> = plan.collect();

    assert_eq!(yielded.len(), generation_order.len());
    let mut sorted = yielded.clone();
    sorted.sort_unstable();
    let mut expected = generation_order.clone();
    expected.sort_unstable();
    assert_eq!(
        sorted, expected,
        "iterator must yield exactly the plan's chunks"
    );
}

#[test]
fn execution_order_is_shuffled_for_large_plans() {
    // a 500-chunk identity permutation from a shuffle is (1/500!) — absence
    // of *any* shuffled plan across 20 seeds means shuffling is broken
    let any_shuffled = (0..20u8).any(|n| {
        let plan = Range::new(0, 100_000)
            .chunk_size(100..=300)
            .seed(seed(n))
            .plan();
        let generation = plan.chunks().to_vec();
        let yielded: Vec<_> = plan.collect();
        yielded != generation
    });
    assert!(
        any_shuffled,
        "execution order never deviated from generation order"
    );
}

// ------------------------------------------------------ start-edge obfuscation

#[test]
fn start_jitter_moves_start_down_only_and_respects_floor() {
    for n in 0..50u8 {
        let plan = Range::new(1000, 2000)
            .start_jitter(500)
            .floor(800)
            .seed(seed(n))
            .plan();
        let start = plan.start();
        assert!(start <= 1000, "jitter must never move the start up");
        assert!(start >= 800, "jitter must respect the floor");
        assert_eq!(plan.end(), 2000, "the end is never extended");
        assert_plan_invariants(&plan, start, 2000);
    }
}

#[test]
fn snapping_collides_clients_within_the_same_interval() {
    let a = Range::new(1042, 2000)
        .snap_start(Snap::Spacing(100))
        .seed(seed(1))
        .plan();
    let b = Range::new(1097, 2000)
        .snap_start(Snap::Spacing(100))
        .seed(seed(2))
        .plan();
    assert_eq!(a.start(), 1000);
    assert_eq!(
        b.start(),
        1000,
        "clients in the same interval must emit identical starts"
    );
}

#[test]
fn snapping_consumes_no_randomness() {
    // same seed, jitter disabled, start already on-grid: enabling snapping
    // must not change a single sample, so the plans are byte-identical
    let with_snap = Range::new(1000, 2000)
        .snap_start(Snap::Spacing(100))
        .seed(seed(7))
        .plan();
    let without_snap = Range::new(1000, 2000).seed(seed(7)).plan();
    assert_eq!(with_snap.chunks(), without_snap.chunks());
    assert_eq!(
        with_snap.collect::<Vec<_>>(),
        without_snap.collect::<Vec<_>>(),
        "snapping must not advance the RNG"
    );
}

#[test]
fn jitter_composed_with_snapping_stays_on_grid() {
    for n in 0..50u8 {
        let plan = Range::new(1042, 2000)
            .start_jitter(500)
            .snap_start(Snap::Spacing(100))
            .seed(seed(n))
            .plan();
        let start = plan.start();
        assert_eq!(start % 100, 0, "emitted start {start} is off-grid");
        assert!(start <= 1000, "start must not exceed the snapped base");
    }
}

#[test]
fn explicit_checkpoint_list_rounds_down() {
    // deliberately unsorted: the builder normalises the list
    let plan = Range::new(1042, 2000)
        .snap_start(Snap::Checkpoints(vec![1500, 100, 419, 1000]))
        .seed(seed(1))
        .plan();
    assert_eq!(plan.start(), 1000);
}

// ---------------------------------------------------------------- determinism

#[test]
fn same_seed_produces_identical_plans() {
    let make = || {
        Range::new(0, 10_000)
            .chunk_size(50..=200)
            .overlap(5..=25)
            .start_jitter(100)
            .seed(seed(99))
            .plan()
    };
    let a = make();
    let b = make();
    assert_eq!(a.chunks(), b.chunks());
    assert_eq!(a.collect::<Vec<_>>(), b.collect::<Vec<_>>());
}

#[test]
fn different_seed_diverges() {
    let a = Range::new(0, 10_000)
        .seed(seed(1))
        .plan()
        .collect::<Vec<_>>();
    let b = Range::new(0, 10_000)
        .seed(seed(2))
        .plan()
        .collect::<Vec<_>>();
    assert_ne!(a, b, "different seeds should produce different plans");
}

// -------------------------------------------------------------- delay laziness

/// The wrapped future must not be polled (and so cannot side-effect) before
/// its scheduled time. Uses paused tokio time: the flag stays unset while the
/// sampled delay has not elapsed.
#[tokio::test(start_paused = true)]
async fn wrapped_future_is_not_polled_before_schedule() {
    let polled = Arc::new(AtomicUsize::new(0));
    let polled_in_task = polled.clone();

    let mut delay = Delay::uniform(Duration::from_secs(5), Duration::from_secs(10)).seed(seed(1));
    let handle = tokio::spawn(async move {
        delay
            .run(async move {
                polled_in_task.fetch_add(1, Ordering::SeqCst);
            })
            .await
    });

    // give the spawned task a chance to start and reach its sleep
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(4)).await;
    tokio::task::yield_now().await;
    assert_eq!(
        polled.load(Ordering::SeqCst),
        0,
        "future ran before the minimum delay elapsed"
    );

    // past the maximum bound the future must have run
    tokio::time::advance(Duration::from_secs(7)).await;
    handle.await.unwrap();
    assert_eq!(polled.load(Ordering::SeqCst), 1);
}

#[tokio::test(start_paused = true)]
async fn run_passes_the_result_through() {
    let mut delay = Delay::uniform(Duration::ZERO, Duration::from_millis(10)).seed(seed(2));
    let out = delay.run(async { 41 + 1 }).await;
    assert_eq!(out, 42);
}

// --------------------------------------------------------------- push drivers

#[tokio::test]
async fn for_each_executes_every_chunk() {
    let plan = Range::new(0, 2000).seed(seed(5)).plan();
    let expected = plan.len();
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    plan.for_each(|_, _| {
        let c = c.clone();
        async move {
            c.fetch_add(1, Ordering::SeqCst);
        }
    })
    .await;
    assert_eq!(count.load(Ordering::SeqCst), expected);
}

#[tokio::test]
async fn for_each_concurrent_respects_limit_and_completes() {
    let plan = Range::new(0, 5000).chunk_size(10..=50).seed(seed(6)).plan();
    let expected = plan.len();
    assert!(
        expected > 8,
        "test needs enough chunks to exercise concurrency"
    );

    let executed = Arc::new(AtomicUsize::new(0));
    let in_flight = Arc::new(AtomicUsize::new(0));
    let max_in_flight = Arc::new(AtomicUsize::new(0));

    let (executed_c, in_flight_c, max_c) =
        (executed.clone(), in_flight.clone(), max_in_flight.clone());
    plan.for_each_concurrent(4, move |_, _| {
        let executed = executed_c.clone();
        let in_flight = in_flight_c.clone();
        let max_in_flight = max_c.clone();
        async move {
            let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            max_in_flight.fetch_max(now, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(2)).await;
            in_flight.fetch_sub(1, Ordering::SeqCst);
            executed.fetch_add(1, Ordering::SeqCst);
        }
    })
    .await;

    assert_eq!(executed.load(Ordering::SeqCst), expected);
    assert!(
        max_in_flight.load(Ordering::SeqCst) <= 4,
        "concurrency limit exceeded: {}",
        max_in_flight.load(Ordering::SeqCst)
    );
}

#[tokio::test]
async fn stream_concurrent_yields_one_output_per_chunk() {
    let make_plan = || {
        Range::new(0, 5000)
            .chunk_size(10..=50)
            .seed(seed(11))
            .plan()
    };
    let mut expected: Vec<(u64, u64)> = make_plan().chunks().to_vec();
    expected.sort_unstable();

    let mut outputs: Vec<(u64, u64)> = make_plan()
        .stream_concurrent(4, |start, end| async move { (start, end) })
        .collect()
        .await;
    outputs.sort_unstable();

    assert_eq!(
        outputs, expected,
        "exactly one output per chunk, no more, no fewer"
    );
}

#[tokio::test]
async fn stream_concurrent_respects_limit() {
    let plan = Range::new(0, 5000)
        .chunk_size(10..=50)
        .seed(seed(12))
        .plan();
    let expected = plan.len();

    let in_flight = Arc::new(AtomicUsize::new(0));
    let max_in_flight = Arc::new(AtomicUsize::new(0));
    let (in_flight_c, max_c) = (in_flight.clone(), max_in_flight.clone());

    let outputs: Vec<()> = plan
        .stream_concurrent(4, move |_, _| {
            let in_flight = in_flight_c.clone();
            let max_in_flight = max_c.clone();
            async move {
                let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                max_in_flight.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(2)).await;
                in_flight.fetch_sub(1, Ordering::SeqCst);
            }
        })
        .collect()
        .await;

    assert_eq!(outputs.len(), expected);
    assert!(
        max_in_flight.load(Ordering::SeqCst) <= 4,
        "concurrency limit exceeded: {}",
        max_in_flight.load(Ordering::SeqCst)
    );
}

#[tokio::test(start_paused = true)]
async fn stream_concurrent_applies_inter_chunk_delays() {
    let plan = Range::new(0, 100)
        .chunk_size(30..=40)
        .seed(seed(13))
        .plan()
        .delay(Delay::uniform(Duration::from_secs(1), Duration::from_secs(2)).seed(seed(13)));
    let chunks = plan.len() as u64;

    let started = tokio::time::Instant::now();
    // limit 1: delays elapse sequentially, so total time lower-bounds at 1s
    // per chunk under auto-advanced paused time
    let _: Vec<()> = plan.stream_concurrent(1, |_, _| async {}).collect().await;
    let elapsed = started.elapsed();

    assert!(
        elapsed >= Duration::from_secs(chunks),
        "expected at least {chunks}s of accumulated delay, got {elapsed:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn inter_chunk_delays_are_applied() {
    let plan = Range::new(0, 100)
        .chunk_size(30..=40)
        .seed(seed(8))
        .plan()
        .delay(Delay::uniform(Duration::from_secs(1), Duration::from_secs(2)).seed(seed(8)));
    let chunks = plan.len() as u64;

    let started = tokio::time::Instant::now();
    plan.for_each(|_, _| async {}).await;
    let elapsed = started.elapsed();

    // each chunk is preceded by a >= 1s delay (auto-advanced paused time)
    assert!(
        elapsed >= Duration::from_secs(chunks),
        "expected at least {chunks}s of accumulated delay, got {elapsed:?}"
    );
}
