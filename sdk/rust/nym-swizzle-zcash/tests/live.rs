// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Opt-in tests against a real lightwalletd — the "don't take our word for
//! it" suite. Gated behind an environment variable (not `#[ignore]`: this
//! repository's CI runs ignored tests as its expensive-test step, and a
//! public lightwalletd should not be part of CI); without the variable the
//! tests skip instantly and touch no network. Run them yourself with:
//!
//! ```sh
//! NYM_SWIZZLE_ZCASH_LIVE_TESTS=1 cargo test -p nym-swizzle-zcash
//!
//! # or against a server you trust more than our default
//! NYM_SWIZZLE_ZCASH_LIVE_TESTS=1 ZEC_SERVER=https://your-lightwalletd:443 \
//!     cargo test -p nym-swizzle-zcash
//! ```
//!
//! They fetch a few thousand compact blocks in total — modest on purpose;
//! please keep it that way when extending them.

#[path = "../examples/support/lightwalletd.rs"]
mod lightwalletd;

use nym_swizzle_zcash::grid::{
    quantize, Disposition, Quantized, QueuedRange, S_FLOOR, VERIFY_LOOKAHEAD,
};
use nym_swizzle_zcash::sync::{self, BlockSource, SyncOutcome};

use crate::lightwalletd::{CompactBlock, Lightwalletd};

/// Stay well behind the tip so a range can't straddle a reorg mid-test.
const TIP_MARGIN: u64 = 1000;

/// The env var that opts into the live suite.
const LIVE_ENV: &str = "NYM_SWIZZLE_ZCASH_LIVE_TESTS";

/// `true` when the live suite is enabled; otherwise prints why the test is
/// skipping and returns `false` (the test then passes as a no-op).
fn live_enabled() -> bool {
    if std::env::var_os(LIVE_ENV).is_some() {
        true
    } else {
        eprintln!("skipped: set {LIVE_ENV}=1 to run the live lightwalletd tests");
        false
    }
}

async fn connect() -> (Lightwalletd, u64) {
    let server =
        std::env::var("ZEC_SERVER").unwrap_or_else(|_| "https://zec.rocks:443".to_string());
    let mut client = Lightwalletd::connect(server.clone())
        .await
        .unwrap_or_else(|e| panic!("cannot reach lightwalletd at {server}: {e}"));
    let tip = client
        .tip()
        .await
        .unwrap_or_else(|e| panic!("{server} refused GetLatestBlock: {e}"));
    (client, tip - TIP_MARGIN)
}

/// The wallet author's slot, wrapping the shared gRPC client and logging
/// what actually goes on the wire.
struct LiveSource {
    client: Lightwalletd,
    requests: Vec<(u64, u64)>,
}

impl LiveSource {
    fn new(client: &Lightwalletd) -> Self {
        Self {
            client: client.clone(),
            requests: Vec::new(),
        }
    }
}

impl BlockSource for LiveSource {
    type Block = CompactBlock;
    type Error = tonic::Status;

    async fn block_range(
        &mut self,
        start: u64,
        end: u64,
    ) -> Result<Vec<(u64, CompactBlock)>, tonic::Status> {
        self.requests.push((start, end));
        let blocks = self.client.block_range(start, end).await?;
        Ok(blocks.into_iter().map(|b| (b.height, b)).collect())
    }
}

/// Real chain data through the verify-window rule: matching stored hashes
/// commit; one corrupted stored hash reads as a reorg. The resume point is
/// deliberately placed within [`VERIFY_LOOKAHEAD`] of a grid boundary, so
/// the one-cell widening rule runs against the live server too.
#[tokio::test]
async fn verify_window_commits_then_detects_reorg_on_real_data() {
    if !live_enabled() {
        return;
    }
    let (mut client, tip) = connect().await;

    // resume 5 blocks above a boundary: closer than the lookahead, so the
    // emitted start must drop one whole cell below `boundary`
    let boundary = (tip - 300) / S_FLOOR * S_FLOOR;
    let resume = boundary + 5;
    let range = QueuedRange::catch_up(resume, tip);
    let quantized = quantize(range, tip);
    assert_eq!(
        quantized.emitted().0,
        boundary - quantized.spacing(),
        "resume point within the lookahead of a boundary must widen one cell"
    );

    // "yesterday's sync": the wallet's stored hashes for the verify window
    let stored: std::collections::BTreeMap<u64, Vec<u8>> = client
        .block_range(resume - VERIFY_LOOKAHEAD, resume)
        .await
        .expect("fetching the stored-state window")
        .into_iter()
        .map(|b| (b.height, b.hash))
        .collect();
    assert_eq!(stored.len(), VERIFY_LOOKAHEAD as usize);

    // clean run: stored state agrees with the chain -> committed
    let mut source = LiveSource::new(&client);
    let outcome = sync::fetch(
        &mut source,
        range,
        tip,
        |_, _, _| {},
        |height, block: &CompactBlock| stored.get(&height) == Some(&block.hash),
    )
    .await
    .expect("live sync failed");
    assert_eq!(outcome, SyncOutcome::Committed);

    // the widened, grid-aligned boundaries really went on the wire
    let min_start = source.requests.iter().map(|r| r.0).min().unwrap();
    assert_eq!(min_start, boundary - quantized.spacing());

    // corrupt one stored hash: the same chain now looks reorged
    let mut corrupted = stored.clone();
    let (&h, _) = corrupted.iter().next().unwrap();
    corrupted.insert(h, vec![0xde, 0xad, 0xbe, 0xef]);

    let outcome = sync::fetch(
        &mut LiveSource::new(&client),
        range,
        tip,
        |_, _, _| {},
        |height, block: &CompactBlock| corrupted.get(&height) == Some(&block.hash),
    )
    .await
    .expect("live sync failed");
    assert_eq!(outcome, SyncOutcome::ReorgDetected);
}

/// Every sync's wire footprint sits on the public grid: requests are
/// ascending, disjoint, S_FLOOR-aligned cells whose union starts on a
/// multiple of the spacing, ends grid-aligned or at the tip, and covers the
/// emitted range exactly — deterministically, so any wallet in the same
/// cell at the same tip sends byte-identical requests.
#[tokio::test]
async fn emitted_boundaries_are_grid_aligned_and_coverage_is_exact() {
    if !live_enabled() {
        return;
    }
    let (client, tip) = connect().await;

    let range = QueuedRange::catch_up(tip - 200, tip);
    let quantized = quantize(range, tip);

    let mut source = LiveSource::new(&client);
    let mut heights = std::collections::BTreeSet::new();
    sync::fetch(
        &mut source,
        range,
        tip,
        |h, _, _| {
            heights.insert(h);
        },
        |_, _| true,
    )
    .await
    .expect("live sync failed");

    let (es, ee) = quantized.emitted();
    let min_start = source.requests.iter().map(|r| r.0).min().unwrap();
    let max_end = source.requests.iter().map(|r| r.1).max().unwrap();

    assert_eq!(min_start % quantized.spacing(), 0, "union start off-grid");
    assert!(
        max_end % quantized.spacing() == 0 || max_end == tip + 1,
        "union end neither grid-aligned nor tip-capped"
    );
    assert_eq!((min_start, max_end), (es, ee));

    // the requests themselves are the deterministic plan: ascending,
    // disjoint, floor-aligned cells
    assert_eq!(source.requests, quantized.requests().collect::<Vec<_>>());
    for &(s, e) in &source.requests {
        assert_eq!(s % S_FLOOR, 0, "request start off the floor grid");
        assert!(e - s <= S_FLOOR);
    }

    assert_eq!(
        heights.len() as u64,
        ee - es,
        "coverage of the emitted range must be exact"
    );
    assert_eq!(*heights.first().unwrap(), es);
    assert_eq!(*heights.last().unwrap(), ee - 1);
}

/// The overhead bound the quantization arithmetic actually guarantees:
/// start extension is below one spacing, plus at most one lookahead
/// widening; the end is tip-capped. A fixed ratio flakes when widening
/// triggers near a spacing boundary.
fn structural_bound(gap: u64, quantized: &Quantized) -> u64 {
    (gap + 1) + quantized.spacing() + VERIFY_LOOKAHEAD
}

/// Measured overhead for the two regimes that occur in practice. The daily
/// incremental sync is fetched live (grid cover, in actual streamed
/// blocks); the long catch-up is arithmetic only, to keep the test polite
/// to the public server. Requests are disjoint, so cover is the only
/// overhead — streamed equals the emitted length exactly.
#[tokio::test]
async fn overhead_measured_for_daily_and_catch_up_regimes() {
    if !live_enabled() {
        return;
    }
    let (client, tip) = connect().await;

    // -- daily incremental: one day behind (one S_FLOOR of blocks) --------
    let gap = S_FLOOR;
    let range = QueuedRange::catch_up(tip - gap, tip);
    let quantized = quantize(range, tip);

    let quant_ratio = quantized.emitted_len() as f64 / (gap + 1) as f64;

    let mut streamed = 0u64;
    let mut wanted = std::collections::BTreeSet::new();
    sync::fetch(
        &mut LiveSource::new(&client),
        range,
        tip,
        |h, _, d| {
            streamed += 1;
            if d == Disposition::Requested {
                wanted.insert(h);
            }
        },
        |_, _| true,
    )
    .await
    .expect("live sync failed");

    println!(
        "daily incremental ({gap}-block gap, spacing {}):",
        quantized.spacing()
    );
    println!(
        "  quantization: {} blocks emitted = {quant_ratio:.2}x the gap; cover is the \
         only overhead",
        quantized.emitted_len()
    );
    assert_eq!(wanted.len() as u64, gap + 1);
    assert_eq!(
        streamed,
        quantized.emitted_len(),
        "requests are disjoint: streamed blocks must equal the emitted length exactly"
    );
    assert!(
        quantized.emitted_len() <= structural_bound(gap, &quantized),
        "daily emitted length {} exceeds the structural bound",
        quantized.emitted_len()
    );

    // -- long catch-up: a month behind (arithmetic only) ------------------
    let gap = 30 * S_FLOOR;
    let quantized = quantize(QueuedRange::catch_up(tip - gap, tip), tip);
    let ratio = quantized.emitted_len() as f64 / (gap + 1) as f64;
    println!(
        "long catch-up ({gap}-block gap, spacing {}):",
        quantized.spacing()
    );
    println!(
        "  quantization: {} blocks emitted = {ratio:.2}x the gap",
        quantized.emitted_len()
    );
    assert!(
        quantized.emitted_len() <= structural_bound(gap, &quantized),
        "catch-up emitted length {} exceeds the structural bound",
        quantized.emitted_len()
    );
}
