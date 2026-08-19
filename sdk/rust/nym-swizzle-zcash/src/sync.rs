// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Executing a quantized range on the wire, through your transport.
//!
//! [`fetch`] quantizes a queued range (see [`grid`](crate::grid)) and drives
//! your [`BlockSource`] through the deterministic request sequence from
//! [`Quantized::requests`]: [`S_FLOOR`](crate::grid::S_FLOOR)-aligned cells,
//! ascending, disjoint, no randomness. Determinism is the point — every
//! wallet resuming in the same grid cell at the same tip emits byte-identical
//! requests, and any per-wallet variation in sizes, order, or overlap would
//! only hand the server a distinguishing dimension while costing bandwidth.
//!
//! Every delivered block reaches your callback tagged with a
//! [`Disposition`]: discard cover below, hash-check the verify window, scan
//! the rest. Every response is checked for **complete delivery** — exactly
//! the heights of its request, no gaps, no duplicates, nothing outside the
//! range — so a censoring or padding server surfaces as
//! [`SyncError::IncompleteDelivery`] instead of a silently short sync. The
//! fetch resolves [`SyncOutcome::Committed`] only once every request has
//! delivered its full range **and** the verify window passed its hash
//! comparison — buffer your scan results and commit them only on that
//! outcome. On a hash mismatch it resolves [`SyncOutcome::ReorgDetected`]:
//! rewind and requeue exactly as your SDK does today.

use std::collections::BTreeSet;
use std::fmt;
use std::ops::ControlFlow;

use futures::StreamExt;

use crate::grid::{classify, quantize, Disposition, Quantized, QueuedRange};

/// Your slot: fetching compact blocks over your own transport.
///
/// The crate performs no network I/O — it only decides *which* ranges go on
/// the wire. Implement this against your existing lightwalletd client (gRPC,
/// proxied, mixnet-tunnelled — your choice). Do **not** reuse the same
/// session for broadcasting transactions; see
/// [`TxBroadcaster`](crate::broadcast::TxBroadcaster).
#[allow(async_fn_in_trait)] // futures are awaited in place by the driver, no Send bound needed
pub trait BlockSource {
    /// Your compact-block type. The crate never looks inside it; it only
    /// needs the height alongside.
    type Block;
    /// Your transport error.
    type Error;

    /// Fetch every block in the half-open height range `[start, end)`,
    /// returning each with its height. (lightwalletd's `BlockRange` is
    /// inclusive on both ends — request `[start, end - 1]` on the wire.)
    ///
    /// Requests are bounded at [`S_FLOOR`](crate::grid::S_FLOOR) (1152)
    /// compact blocks — roughly 1–2 MB — so buffering one response in a
    /// `Vec` is bounded regardless of how far behind the wallet is.
    async fn block_range(
        &mut self,
        start: u64,
        end: u64,
    ) -> Result<Vec<(u64, Self::Block)>, Self::Error>;
}

/// How a completed sync resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOutcome {
    /// All requests landed and the verify window (if any) matched stored
    /// state: commit your buffered scan results.
    Committed,
    /// A verify-window block's hash did not match stored state: a reorg
    /// happened below the resume point. Discard buffered results, rewind and
    /// requeue as your SDK does today.
    ReorgDetected,
}

/// Why a sync could not complete.
#[derive(Debug)]
pub enum SyncError<E> {
    /// A request failed in your [`BlockSource`].
    Source(E),
    /// A response did not deliver exactly its requested range: heights were
    /// missing, duplicated, or outside the request. A censored or padded
    /// response can never reach [`SyncOutcome::Committed`].
    IncompleteDelivery {
        /// The half-open request whose response was deficient.
        request: (u64, u64),
        /// Requested heights that never arrived.
        missing: u64,
        /// Delivered heights that were duplicates or outside the request.
        unexpected: u64,
    },
    /// The verify window never completed even though every request passed
    /// delivery verification. Unreachable when the emitted range contains
    /// the window (which quantization guarantees); kept as a defensive
    /// backstop so a future planning bug fails loudly instead of committing.
    VerifyWindowIncomplete {
        /// Verify-window heights that were never delivered.
        missing: Vec<u64>,
    },
}

impl<E: fmt::Display> fmt::Display for SyncError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncError::Source(e) => write!(f, "block source failed: {e}"),
            SyncError::IncompleteDelivery {
                request: (start, end),
                missing,
                unexpected,
            } => write!(
                f,
                "response for {start}..{end} did not deliver exactly the requested range \
                 ({missing} height(s) missing, {unexpected} duplicate/out-of-range); cannot commit"
            ),
            SyncError::VerifyWindowIncomplete { missing } => write!(
                f,
                "server never delivered {} verify-window height(s) ({missing:?}); cannot commit",
                missing.len()
            ),
        }
    }
}

impl<E: fmt::Display + fmt::Debug> std::error::Error for SyncError<E> {}

/// Verify a response delivered exactly the heights of its half-open request:
/// no gaps, no duplicates, nothing outside the range. Order-independent; the
/// bookkeeping is one flag per requested height, bounded by the wire-split
/// unit ([`S_FLOOR`](crate::grid::S_FLOOR)).
fn verify_delivery<B, E>(request: (u64, u64), blocks: &[(u64, B)]) -> Result<(), SyncError<E>> {
    let (start, end) = request;
    let mut seen = vec![false; (end - start) as usize];
    let mut unexpected = 0u64;
    for (height, _) in blocks {
        match height.checked_sub(start).map(|i| i as usize) {
            Some(i) if i < seen.len() && !seen[i] => seen[i] = true,
            _ => unexpected += 1,
        }
    }
    let missing = seen.iter().filter(|delivered| !**delivered).count() as u64;
    if missing == 0 && unexpected == 0 {
        Ok(())
    } else {
        Err(SyncError::IncompleteDelivery {
            request,
            missing,
            unexpected,
        })
    }
}

/// Quantize `range` and fetch it through `source`, one request at a time, in
/// ascending order.
///
/// Every block is handed to `on_block(height, block, disposition)`. Blocks
/// tagged [`Disposition::VerifyWindow`] are *first* passed to
/// `check_hash(height, &block)`, which must answer whether the block matches
/// the hash your wallet has stored for that height; on the first mismatch
/// the sync stops issuing requests and resolves
/// [`SyncOutcome::ReorgDetected`].
pub async fn fetch<S, F, V>(
    source: &mut S,
    range: QueuedRange,
    tip: u64,
    mut on_block: F,
    mut check_hash: V,
) -> Result<SyncOutcome, SyncError<S::Error>>
where
    S: BlockSource,
    F: FnMut(u64, S::Block, Disposition),
    V: FnMut(u64, &S::Block) -> bool,
{
    let quantized = quantize(range, tip);
    let mut window = WindowTracker::new(&quantized);

    for (start, end) in quantized.requests() {
        let blocks = source
            .block_range(start, end)
            .await
            .map_err(SyncError::Source)?;
        verify_delivery((start, end), &blocks)?;
        if let ControlFlow::Break(()) =
            window.absorb(blocks, &quantized, &mut on_block, &mut check_hash)
        {
            return Ok(SyncOutcome::ReorgDetected);
        }
    }

    window.conclude()
}

/// Like [`fetch`], with up to `limit` requests in flight at once. Each
/// in-flight request runs on its own clone of `source` (a lightwalletd
/// channel clone is cheap and multiplexes over one connection); callbacks
/// still run sequentially, in arrival order. The request *sequence* is the
/// same deterministic one as [`fetch`] — concurrency changes completion
/// order, not what goes on the wire.
pub async fn fetch_concurrent<S, F, V>(
    source: &S,
    limit: usize,
    range: QueuedRange,
    tip: u64,
    mut on_block: F,
    mut check_hash: V,
) -> Result<SyncOutcome, SyncError<S::Error>>
where
    S: BlockSource + Clone,
    F: FnMut(u64, S::Block, Disposition),
    V: FnMut(u64, &S::Block) -> bool,
{
    let quantized = quantize(range, tip);
    let mut window = WindowTracker::new(&quantized);

    let mut results = futures::stream::iter(quantized.requests().map(|(start, end)| {
        let mut source = source.clone();
        async move { ((start, end), source.block_range(start, end).await) }
    }))
    // lower clamp: `limit.max(1)` raises a zero limit to 1 (futures'
    // concurrency adapters panic on 0); larger limits pass through
    .buffer_unordered(limit.max(1));

    while let Some((request, result)) = results.next().await {
        let blocks = result.map_err(SyncError::Source)?;
        verify_delivery(request, &blocks)?;
        if let ControlFlow::Break(()) =
            window.absorb(blocks, &quantized, &mut on_block, &mut check_hash)
        {
            // dropping the stream cancels in-flight requests; the reorg is
            // getting re-synced anyway
            return Ok(SyncOutcome::ReorgDetected);
        }
    }

    window.conclude()
}

/// Tracks which verify-window heights have arrived and whether they matched.
struct WindowTracker {
    remaining: BTreeSet<u64>,
}

impl WindowTracker {
    fn new(quantized: &Quantized) -> Self {
        let remaining = quantized
            .verify_window()
            .map(|(ws, we)| (ws..we).collect())
            .unwrap_or_default();
        Self { remaining }
    }

    /// Deliver one request's blocks; breaks on a verify mismatch. Every
    /// block received up to and including the mismatching one reaches
    /// `on_block`.
    fn absorb<B>(
        &mut self,
        blocks: Vec<(u64, B)>,
        quantized: &Quantized,
        on_block: &mut impl FnMut(u64, B, Disposition),
        check_hash: &mut impl FnMut(u64, &B) -> bool,
    ) -> ControlFlow<()> {
        for (height, block) in blocks {
            let disposition = classify(height, quantized);
            let mut matches = true;
            if disposition == Disposition::VerifyWindow {
                self.remaining.remove(&height);
                matches = check_hash(height, &block);
            }
            on_block(height, block, disposition);
            if !matches {
                return ControlFlow::Break(());
            }
        }
        ControlFlow::Continue(())
    }

    fn conclude<E>(self) -> Result<SyncOutcome, SyncError<E>> {
        if self.remaining.is_empty() {
            Ok(SyncOutcome::Committed)
        } else {
            Err(SyncError::VerifyWindowIncomplete {
                missing: self.remaining.into_iter().collect(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::{BTreeMap, BTreeSet};
    use std::rc::Rc;

    use super::*;
    use crate::grid::{S_FLOOR, VERIFY_LOOKAHEAD};

    /// An in-memory chain: block "hash" is a function of height, with an
    /// optional reorg point above which hashes differ.
    #[derive(Clone)]
    struct FakeChain {
        tip: u64,
        reorged_from: Option<u64>,
        requests: Rc<RefCell<Vec<(u64, u64)>>>,
        fail_on: Option<(u64, u64)>,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct FakeBlock {
        hash: u64,
    }

    impl FakeChain {
        fn new(tip: u64) -> Self {
            Self {
                tip,
                reorged_from: None,
                requests: Rc::new(RefCell::new(Vec::new())),
                fail_on: None,
            }
        }

        fn hash_at(&self, height: u64) -> u64 {
            match self.reorged_from {
                Some(fork) if height >= fork => height.wrapping_mul(31) + 7,
                _ => height.wrapping_mul(31),
            }
        }
    }

    impl BlockSource for FakeChain {
        type Block = FakeBlock;
        type Error = String;

        async fn block_range(
            &mut self,
            start: u64,
            end: u64,
        ) -> Result<Vec<(u64, FakeBlock)>, String> {
            self.requests.borrow_mut().push((start, end));
            if self.fail_on == Some((start, end)) {
                return Err("boom".into());
            }
            assert!(end <= self.tip + 1, "request past the tip");
            Ok((start..end)
                .map(|h| {
                    (
                        h,
                        FakeBlock {
                            hash: self.hash_at(h),
                        },
                    )
                })
                .collect())
        }
    }

    const TIP: u64 = 2_000_000;
    const RESUME: u64 = 1_999_000;

    /// Stored state agreeing with the pre-reorg chain.
    fn stored_hashes() -> BTreeMap<u64, u64> {
        let clean = FakeChain::new(TIP);
        (RESUME - VERIFY_LOOKAHEAD..RESUME)
            .map(|h| (h, clean.hash_at(h)))
            .collect()
    }

    fn check_against(stored: BTreeMap<u64, u64>) -> impl FnMut(u64, &FakeBlock) -> bool {
        move |height, block| stored.get(&height) == Some(&block.hash)
    }

    #[tokio::test]
    async fn clean_chain_commits_and_covers_the_emitted_range() {
        let mut chain = FakeChain::new(TIP);
        let received = Rc::new(RefCell::new(BTreeSet::new()));
        let r2 = received.clone();

        let outcome = fetch(
            &mut chain,
            QueuedRange::catch_up(RESUME, TIP),
            TIP,
            move |h, _, _| {
                r2.borrow_mut().insert(h);
            },
            check_against(stored_hashes()),
        )
        .await
        .unwrap();

        assert_eq!(outcome, SyncOutcome::Committed);

        // every emitted height arrives exactly once: requests are disjoint
        let q = quantize(QueuedRange::catch_up(RESUME, TIP), TIP);
        let (es, ee) = q.emitted();
        let received = received.borrow();
        assert_eq!(*received.first().unwrap(), es);
        assert_eq!(*received.last().unwrap(), ee - 1);
        assert_eq!(received.len() as u64, ee - es, "gap in coverage");

        // the wire requests are exactly the deterministic plan
        assert_eq!(*chain.requests.borrow(), q.requests().collect::<Vec<_>>());
    }

    #[tokio::test]
    async fn requests_are_deterministic_ascending_and_floor_aligned() {
        let range = QueuedRange::catch_up(RESUME, TIP);

        let mut runs = Vec::new();
        for _ in 0..2 {
            let mut chain = FakeChain::new(TIP);
            fetch(&mut chain, range, TIP, |_, _, _| {}, |_, _| true)
                .await
                .unwrap();
            runs.push(chain.requests.borrow().clone());
        }
        assert_eq!(
            runs[0], runs[1],
            "identical wallet state must emit identical requests — no randomness"
        );

        for window in runs[0].windows(2) {
            assert!(window[0].1 == window[1].0, "requests must ascend gaplessly");
        }
        for &(s, e) in &runs[0] {
            assert_eq!(s % S_FLOOR, 0);
            assert!(e - s <= S_FLOOR);
        }
    }

    #[tokio::test]
    async fn reorged_chain_is_detected() {
        let mut chain = FakeChain::new(TIP);
        chain.reorged_from = Some(RESUME - 3); // fork inside the verify window

        let outcome = fetch(
            &mut chain,
            QueuedRange::catch_up(RESUME, TIP),
            TIP,
            |_, _, _| {},
            check_against(stored_hashes()),
        )
        .await
        .unwrap();

        assert_eq!(outcome, SyncOutcome::ReorgDetected);
    }

    #[tokio::test]
    async fn reorg_stops_further_requests() {
        let mut chain = FakeChain::new(TIP);
        chain.reorged_from = Some(RESUME - VERIFY_LOOKAHEAD);
        let requests = chain.requests.clone();

        let outcome = fetch(
            &mut chain,
            QueuedRange::catch_up(RESUME, TIP),
            TIP,
            |_, _, _| {},
            check_against(stored_hashes()),
        )
        .await
        .unwrap();
        assert_eq!(outcome, SyncOutcome::ReorgDetected);

        let issued = requests.borrow().len();
        let all = quantize(QueuedRange::catch_up(RESUME, TIP), TIP)
            .requests()
            .count();
        assert!(
            issued < all,
            "driver should stop issuing requests after the mismatch ({issued} of {all})"
        );
    }

    /// A source that tampers with an honest response: drops a height range,
    /// duplicates a block, or appends one from outside the request.
    #[derive(Clone)]
    struct Tampering {
        chain: FakeChain,
        drop_heights: Option<(u64, u64)>,
        duplicate_first: bool,
        append_out_of_range: bool,
    }

    impl Tampering {
        fn new(chain: FakeChain) -> Self {
            Self {
                chain,
                drop_heights: None,
                duplicate_first: false,
                append_out_of_range: false,
            }
        }
    }

    impl BlockSource for Tampering {
        type Block = FakeBlock;
        type Error = String;
        async fn block_range(
            &mut self,
            start: u64,
            end: u64,
        ) -> Result<Vec<(u64, FakeBlock)>, String> {
            let mut blocks = self.chain.block_range(start, end).await?;
            if let Some((ds, de)) = self.drop_heights {
                blocks.retain(|(h, _)| !(ds..de).contains(h));
            }
            if self.duplicate_first {
                let first = blocks.first().cloned().expect("nonempty response");
                blocks.push(first);
            }
            if self.append_out_of_range {
                blocks.push((end + 500, FakeBlock { hash: 0 }));
            }
            Ok(blocks)
        }
    }

    async fn expect_incomplete_delivery(source: Tampering) -> SyncError<String> {
        fetch(
            &mut source.clone(),
            QueuedRange::catch_up(RESUME, TIP),
            TIP,
            |_, _, _| {},
            |_, _| true,
        )
        .await
        .expect_err("tampered delivery must not complete")
    }

    #[tokio::test]
    async fn censored_verify_window_is_incomplete_delivery() {
        let mut source = Tampering::new(FakeChain::new(TIP));
        source.drop_heights = Some((RESUME - VERIFY_LOOKAHEAD, RESUME));

        match expect_incomplete_delivery(source).await {
            SyncError::IncompleteDelivery {
                missing,
                unexpected,
                ..
            } => {
                assert_eq!(missing, VERIFY_LOOKAHEAD);
                assert_eq!(unexpected, 0);
            }
            other => panic!("expected IncompleteDelivery, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn censored_requested_heights_are_incomplete_delivery() {
        // heights the wallet actually asked for, well above the verify window
        let mut source = Tampering::new(FakeChain::new(TIP));
        source.drop_heights = Some((RESUME + 100, RESUME + 105));

        match expect_incomplete_delivery(source).await {
            SyncError::IncompleteDelivery { missing, .. } => assert_eq!(missing, 5),
            other => panic!("expected IncompleteDelivery, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn censored_cover_heights_are_incomplete_delivery() {
        // cover-only heights: the emitted range is the contract, not just
        // the wallet's requested sub-range
        let q = quantize(QueuedRange::catch_up(RESUME, TIP), TIP);
        let (es, _) = q.emitted();
        assert!(es < RESUME - VERIFY_LOOKAHEAD, "test needs cover below");
        let mut source = Tampering::new(FakeChain::new(TIP));
        source.drop_heights = Some((es, es + 3));

        match expect_incomplete_delivery(source).await {
            SyncError::IncompleteDelivery { missing, .. } => assert_eq!(missing, 3),
            other => panic!("expected IncompleteDelivery, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn duplicate_and_out_of_range_heights_are_unexpected() {
        let mut source = Tampering::new(FakeChain::new(TIP));
        source.duplicate_first = true;
        source.append_out_of_range = true;

        match expect_incomplete_delivery(source).await {
            SyncError::IncompleteDelivery {
                missing,
                unexpected,
                ..
            } => {
                assert_eq!(missing, 0);
                assert_eq!(unexpected, 2);
            }
            other => panic!("expected IncompleteDelivery, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn concurrent_fetch_also_enforces_delivery() {
        let mut source = Tampering::new(FakeChain::new(TIP));
        source.drop_heights = Some((RESUME + 100, RESUME + 101));

        let err = fetch_concurrent(
            &source,
            4,
            QueuedRange::catch_up(RESUME, TIP),
            TIP,
            |_, _, _| {},
            |_, _| true,
        )
        .await
        .expect_err("tampered delivery must not complete");
        assert!(matches!(err, SyncError::IncompleteDelivery { .. }));
    }

    #[tokio::test]
    async fn source_errors_propagate() {
        let mut chain = FakeChain::new(TIP);
        let q = quantize(QueuedRange::catch_up(RESUME, TIP), TIP);
        chain.fail_on = q.requests().next();

        let err = fetch(
            &mut chain,
            QueuedRange::catch_up(RESUME, TIP),
            TIP,
            |_, _, _| {},
            |_, _| true,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, SyncError::Source(_)));
    }

    #[tokio::test]
    async fn dispositions_partition_the_emitted_range() {
        let mut chain = FakeChain::new(TIP);
        let seen: Rc<RefCell<BTreeMap<Disposition, BTreeSet<u64>>>> = Rc::default();
        let s2 = seen.clone();

        let range = QueuedRange::catch_up(RESUME, TIP);
        fetch(
            &mut chain,
            range,
            TIP,
            move |h, _, d| {
                s2.borrow_mut().entry(d).or_default().insert(h);
            },
            check_against(stored_hashes()),
        )
        .await
        .unwrap();

        let q = quantize(range, TIP);
        let seen = seen.borrow();
        let count = |d: Disposition| seen.get(&d).map_or(0, |s| s.len() as u64);

        // catch-up to tip: no cover above; cover below fills the rest
        assert_eq!(count(Disposition::Requested), TIP + 1 - RESUME);
        assert_eq!(count(Disposition::VerifyWindow), VERIFY_LOOKAHEAD);
        assert_eq!(count(Disposition::CoverAbove), 0);
        assert_eq!(
            count(Disposition::CoverBelow),
            q.emitted_len() - (TIP + 1 - RESUME) - VERIFY_LOOKAHEAD
        );
        // each height classifies exactly one way, and requests are disjoint,
        // so the buckets partition the emitted range with nothing left over
        let mut union = BTreeSet::new();
        for set in seen.values() {
            assert!(union.is_disjoint(set), "a height classified two ways");
            union.extend(set);
        }
        assert_eq!(union.len() as u64, q.emitted_len());
    }

    #[tokio::test]
    async fn concurrent_fetch_matches_sequential_coverage() {
        let range = QueuedRange::catch_up(RESUME, TIP);
        let chain = FakeChain::new(TIP);
        let received = Rc::new(RefCell::new(BTreeSet::new()));
        let r2 = received.clone();

        let outcome = fetch_concurrent(
            &chain,
            4,
            range,
            TIP,
            move |h, _, _| {
                r2.borrow_mut().insert(h);
            },
            check_against(stored_hashes()),
        )
        .await
        .unwrap();
        assert_eq!(outcome, SyncOutcome::Committed);

        let q = quantize(range, TIP);
        let (es, ee) = q.emitted();
        assert_eq!(received.borrow().len() as u64, ee - es);

        // concurrency changes completion order, not what goes on the wire:
        // the request *set* is the same deterministic plan
        let mut issued = chain.requests.borrow().clone();
        issued.sort_unstable();
        assert_eq!(issued, q.requests().collect::<Vec<_>>());
    }
}
