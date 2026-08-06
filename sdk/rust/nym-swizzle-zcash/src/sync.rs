// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Executing a quantized range as overlapping, shuffled chunks, through your
//! transport.
//!
//! [`SyncSession::fetch`] quantizes a queued range (see [`grid`](crate::grid)),
//! decomposes the emitted range into randomly sized, deliberately overlapping
//! chunks in shuffled order (via [`nym_swizzle::Range`]), and drives your
//! [`BlockSource`] to fetch them — so no single request on the wire describes
//! either your resume point or your actual interval.
//!
//! Every delivered block reaches your callback tagged with a
//! [`Disposition`]: discard cover below, hash-check the verify window, scan
//! the rest. The session resolves [`SyncOutcome::Committed`] only once every
//! chunk has landed **and** the verify window passed its hash comparison —
//! buffer your scan results and commit them only on that outcome. On a hash
//! mismatch it resolves [`SyncOutcome::ReorgDetected`]: rewind and requeue
//! exactly as your SDK does today.
//!
//! The driver never reorders chunks to fetch the verify window early — a
//! predictable "verify first" shape would be identifying. The check simply
//! runs whenever the window's chunk happens to arrive.

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
    async fn block_range(
        &mut self,
        start: u64,
        end: u64,
    ) -> Result<Vec<(u64, Self::Block)>, Self::Error>;
}

/// How a completed sync resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOutcome {
    /// All chunks landed and the verify window (if any) matched stored
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
    /// A chunk fetch failed in your [`BlockSource`].
    Source(E),
    /// The server never delivered some verify-window heights, so the reorg
    /// check cannot conclude and committing is not allowed.
    VerifyWindowIncomplete {
        /// Verify-window heights that were never delivered.
        missing: Vec<u64>,
    },
}

impl<E: fmt::Display> fmt::Display for SyncError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncError::Source(e) => write!(f, "block source failed: {e}"),
            SyncError::VerifyWindowIncomplete { missing } => write!(
                f,
                "server never delivered {} verify-window height(s) ({missing:?}); cannot commit",
                missing.len()
            ),
        }
    }
}

impl<E: fmt::Display + fmt::Debug> std::error::Error for SyncError<E> {}

/// A configured sync driver. One instance can run many fetches.
#[derive(Debug, Default, Clone)]
pub struct SyncSession {
    seed: Option<[u8; 32]>,
}

impl SyncSession {
    /// A driver with operating-system randomness (what you want in
    /// production).
    pub fn new() -> Self {
        Self::default()
    }

    /// Use a deterministic seed: the same seed, range and tip produce an
    /// identical chunk plan (sizes, overlaps and order). For tests and
    /// reproducing traffic shapes — not for production.
    pub fn seed(mut self, seed: [u8; 32]) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Quantize `range` and fetch it through `source`, one chunk at a time.
    ///
    /// Every block is handed to `on_block(height, block, disposition)`.
    /// Blocks tagged [`Disposition::VerifyWindow`] are *first* passed to
    /// `check_hash(height, &block)`, which must answer whether the block
    /// matches the hash your wallet has stored for that height; on the first
    /// mismatch the sync stops issuing requests and resolves
    /// [`SyncOutcome::ReorgDetected`].
    pub async fn fetch<S, F, V>(
        &self,
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

        for (start, end) in self.plan(&quantized) {
            let blocks = source
                .block_range(start, end)
                .await
                .map_err(SyncError::Source)?;
            if let ControlFlow::Break(()) =
                window.absorb(blocks, &quantized, &mut on_block, &mut check_hash)
            {
                return Ok(SyncOutcome::ReorgDetected);
            }
        }

        window.conclude()
    }

    /// Like [`fetch`](Self::fetch), with up to `limit` chunk requests in
    /// flight at once. Each in-flight request runs on its own clone of
    /// `source` (a lightwalletd channel clone is cheap and multiplexes over
    /// one connection); callbacks still run sequentially, in arrival order.
    pub async fn fetch_concurrent<S, F, V>(
        &self,
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

        let mut results = futures::stream::iter(self.plan(&quantized).map(|(start, end)| {
            let mut source = source.clone();
            async move { source.block_range(start, end).await }
        }))
        .buffer_unordered(limit.max(1));

        while let Some(result) = results.next().await {
            let blocks = result.map_err(SyncError::Source)?;
            if let ControlFlow::Break(()) =
                window.absorb(blocks, &quantized, &mut on_block, &mut check_hash)
            {
                // dropping the stream cancels in-flight chunks; the reorg is
                // getting re-synced anyway
                return Ok(SyncOutcome::ReorgDetected);
            }
        }

        window.conclude()
    }

    /// The shuffled, overlapping chunk plan over the emitted range.
    fn plan(&self, quantized: &Quantized) -> nym_swizzle::ChunkPlan {
        let (start, end) = quantized.emitted();
        let mut range = nym_swizzle::Range::new(start, end);
        if let Some(seed) = self.seed {
            range = range.seed(seed);
        }
        range.plan()
    }
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

    /// Deliver one chunk's blocks; breaks on a verify mismatch. Every block
    /// received up to and including the mismatching one reaches `on_block`.
    fn absorb<B>(
        &mut self,
        blocks: Vec<(u64, B)>,
        quantized: &Quantized,
        on_block: &mut impl FnMut(u64, B, Disposition),
        check_hash: &mut impl FnMut(u64, &B) -> bool,
    ) -> ControlFlow<()> {
        for (height, block) in blocks {
            let disposition = classify(height, quantized);
            let matches = disposition != Disposition::VerifyWindow || {
                self.remaining.remove(&height);
                check_hash(height, &block)
            };
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
    use crate::grid::VERIFY_LOOKAHEAD;

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

        let outcome = SyncSession::new()
            .seed([1; 32])
            .fetch(
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

        // the union of requests covers the emitted range exactly
        let q = quantize(QueuedRange::catch_up(RESUME, TIP), TIP);
        let (es, ee) = q.emitted();
        let received = received.borrow();
        assert_eq!(*received.first().unwrap(), es);
        assert_eq!(*received.last().unwrap(), ee - 1);
        assert_eq!(received.len() as u64, ee - es, "gap in coverage");

        // no request spills outside the emitted range
        for &(s, e) in chain.requests.borrow().iter() {
            assert!(
                s >= es && e <= ee,
                "chunk {s}..{e} spills out of {es}..{ee}"
            );
        }
    }

    #[tokio::test]
    async fn reorged_chain_is_detected() {
        let mut chain = FakeChain::new(TIP);
        chain.reorged_from = Some(RESUME - 3); // fork inside the verify window

        let outcome = SyncSession::new()
            .seed([1; 32])
            .fetch(
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

        let outcome = SyncSession::new()
            .seed([7; 32])
            .fetch(
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
        let total = quantize(QueuedRange::catch_up(RESUME, TIP), TIP);
        let all = SyncSession::new().seed([7; 32]).plan(&total).len();
        assert!(
            issued < all,
            "driver should stop issuing chunks after the mismatch ({issued} of {all})"
        );
    }

    #[tokio::test]
    async fn commit_requires_complete_verify_window() {
        /// A source that silently drops the verify-window heights.
        #[derive(Clone)]
        struct Censoring(FakeChain);

        impl BlockSource for Censoring {
            type Block = FakeBlock;
            type Error = String;
            async fn block_range(
                &mut self,
                start: u64,
                end: u64,
            ) -> Result<Vec<(u64, FakeBlock)>, String> {
                let blocks = self.0.block_range(start, end).await?;
                Ok(blocks
                    .into_iter()
                    .filter(|(h, _)| !(RESUME - VERIFY_LOOKAHEAD..RESUME).contains(h))
                    .collect())
            }
        }

        let err = SyncSession::new()
            .fetch(
                &mut Censoring(FakeChain::new(TIP)),
                QueuedRange::catch_up(RESUME, TIP),
                TIP,
                |_, _, _| {},
                |_, _| true,
            )
            .await
            .unwrap_err();

        match err {
            SyncError::VerifyWindowIncomplete { missing } => {
                assert_eq!(missing.len(), VERIFY_LOOKAHEAD as usize)
            }
            other => panic!("expected VerifyWindowIncomplete, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn source_errors_propagate() {
        let mut chain = FakeChain::new(TIP);
        // fail the first chunk the plan will issue
        let q = quantize(QueuedRange::catch_up(RESUME, TIP), TIP);
        let first = SyncSession::new().seed([2; 32]).plan(&q).next().unwrap();
        chain.fail_on = Some(first);

        let err = SyncSession::new()
            .seed([2; 32])
            .fetch(
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
        SyncSession::new()
            .fetch(
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
        // overlapping chunks re-deliver blocks; a set per disposition proves
        // each height classifies consistently — no height in two buckets
        let mut union = BTreeSet::new();
        for set in seen.values() {
            assert!(union.is_disjoint(set), "a height classified two ways");
            union.extend(set);
        }
    }

    #[tokio::test]
    async fn seeded_sync_is_reproducible() {
        let range = QueuedRange::catch_up(RESUME, TIP);

        let mut runs = Vec::new();
        for _ in 0..2 {
            let mut chain = FakeChain::new(TIP);
            SyncSession::new()
                .seed([9; 32])
                .fetch(&mut chain, range, TIP, |_, _, _| {}, |_, _| true)
                .await
                .unwrap();
            runs.push(chain.requests.borrow().clone());
        }
        assert_eq!(runs[0], runs[1], "same seed must replay the same requests");

        let mut chain = FakeChain::new(TIP);
        SyncSession::new()
            .seed([10; 32])
            .fetch(&mut chain, range, TIP, |_, _, _| {}, |_, _| true)
            .await
            .unwrap();
        assert_ne!(
            runs[0],
            *chain.requests.borrow(),
            "different seed should diverge"
        );
    }

    #[tokio::test]
    async fn concurrent_fetch_matches_sequential_coverage() {
        let range = QueuedRange::catch_up(RESUME, TIP);
        let chain = FakeChain::new(TIP);
        let received = Rc::new(RefCell::new(BTreeSet::new()));
        let r2 = received.clone();

        let outcome = SyncSession::new()
            .seed([3; 32])
            .fetch_concurrent(
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
        // clones share the request log through the Rc
        assert!(!chain.requests.borrow().is_empty());
    }
}
