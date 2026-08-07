// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Decoupled broadcast scheduling: don't send when you sync.
//!
//! A wallet that broadcasts right after syncing hands the server a link
//! between the transaction and the sync session — and through the sync
//! session, potentially the wallet's whole history. The fix is to delay the
//! broadcast by an amount calibrated to the *network's* transaction arrival
//! rate, not to anything about your wallet, so the send disappears into
//! everyone else's traffic (the same distribution and parameters as ZIP 318
//! transfer scheduling, so wallet broadcasts pool with migration traffic).
//!
//! Delays run for hours, and a phone won't keep your process alive that
//! long. So scheduling is two-phase: [`Scheduler::schedule`] samples the
//! delay **once** and hands you a [`BroadcastPlan`] — plain data you persist
//! however you like. After a restart, [`BroadcastPlan::resume`] waits out
//! only the remaining time, then invokes your transaction *builder* and hands
//! the result to your [`TxBroadcaster`]. Building after the delay matters:
//! the transaction's expiry height is visible on-chain, and deriving it from
//! a pre-delay tip would tell everyone when you last synced. Build with a
//! fresh tip; [`expiry_height`] does the arithmetic.
//!
//! You usually don't need to sync first. Consensus accepts old anchors, and
//! ZIP 318 grid anchors are retained for about two days — check with
//! [`needs_refresh_sync`], and only when it says so, run a sync (on its own
//! session) and put the broadcast on a later timer. Never broadcast over the
//! connection you sync on; ideally use a different server entirely.

use std::future::Future;
use std::time::Duration;

use nym_swizzle::Delay;

/// Zcash's target block time (post-Blossom).
pub const TARGET_BLOCK_TIME: Duration = Duration::from_secs(75);

/// The expiry delta wallets attach to transactions
/// (`DEFAULT_TX_EXPIRY_DELTA` in librustzcash): expiry = tip + 40.
pub const DEFAULT_TX_EXPIRY_DELTA: u64 = 40;

/// How long ZIP 318 grid anchors are retained, in blocks (about two days).
/// While the wallet's last sync is younger than this, it can build and send
/// with no preceding request.
pub const ZIP318_ANCHOR_RETENTION: u64 = 2 * 1152;

/// Convert a block count to wall-clock time at the target block rate, so
/// delay parameters read in blocks, as the protocol literature writes them.
pub const fn blocks(n: u64) -> Duration {
    Duration::from_secs(n * TARGET_BLOCK_TIME.as_secs())
}

/// The expiry height for a transaction built now: `fresh_tip + 40`.
///
/// `fresh_tip` must be fetched *after* the broadcast delay has elapsed —
/// expiry is visible on-chain, and a stale tip plus 40 reveals your last
/// sync height to everyone.
pub const fn expiry_height(fresh_tip: u64) -> u64 {
    fresh_tip + DEFAULT_TX_EXPIRY_DELTA
}

/// Whether a broadcast needs a refresh sync first: `true` once the last
/// synced height has fallen more than the anchor-retention bound behind the
/// tip. If it has, sync on its own session and schedule the broadcast on a
/// later timer; if not, build and send with no preceding request.
pub const fn needs_refresh_sync(last_synced_height: u64, tip: u64) -> bool {
    tip.saturating_sub(last_synced_height) > ZIP318_ANCHOR_RETENTION
}

/// Which delay distribution a broadcast was scheduled with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Profile {
    /// Exponential with mean 144 blocks (~3 h), resampled above 576 blocks
    /// (~12 h). The ZIP 318 transfer-scheduling parameters: your broadcast
    /// pools with an anonymity set of roughly 120 comparable transactions.
    Standard,
    /// Exponential with mean 24 blocks (~30 min), resampled above 96 blocks
    /// (~2 h). Faster, but the anonymity set shrinks to roughly 20
    /// comparable transactions.
    Fast,
}

impl Profile {
    /// Mean delay in blocks.
    pub const fn mean_blocks(self) -> u64 {
        match self {
            Profile::Standard => 144,
            Profile::Fast => 24,
        }
    }

    /// Hard cap in blocks; samples above it are re-drawn, never clamped
    /// (clamping would pile broadcasts up at the cap — a recognisable spike).
    pub const fn cap_blocks(self) -> u64 {
        match self {
            Profile::Standard => 576,
            Profile::Fast => 96,
        }
    }
}

/// Samples broadcast delays. Construct once per send with
/// [`standard`](Scheduler::standard) or [`fast`](Scheduler::fast).
#[derive(Debug)]
pub struct Scheduler {
    profile: Profile,
    delay: Delay,
}

impl Scheduler {
    /// The default profile — use this unless the user explicitly opts into
    /// faster, less private sends. Keep the parameters as they are: a custom
    /// delay distribution is a fingerprint.
    pub fn standard() -> Self {
        Self::with_profile(Profile::Standard)
    }

    /// The opt-in fast profile. See [`Profile::Fast`] for what it costs.
    pub fn fast() -> Self {
        Self::with_profile(Profile::Fast)
    }

    fn with_profile(profile: Profile) -> Self {
        let delay = Delay::poisson(blocks(profile.mean_blocks())).max(blocks(profile.cap_blocks()));
        Self { profile, delay }
    }

    /// Use a deterministic seed for the delay sample. For tests — not for
    /// production sends.
    pub fn seed(mut self, seed: [u8; 32]) -> Self {
        self.delay = self.delay.seed(seed);
        self
    }

    /// Sample the delay — exactly once — and return the plan to persist.
    ///
    /// Persist it together with the moment you scheduled (your clock, your
    /// storage), so that after a restart you can tell [`resume`] how much
    /// time has already passed.
    pub fn schedule(&mut self) -> BroadcastPlan {
        BroadcastPlan {
            delay_secs: self.delay.sample().as_secs(),
            profile: self.profile,
        }
    }
}

/// Your slot: sending a raw transaction over your own transport.
///
/// Keep it separate from your [`BlockSource`](crate::sync::BlockSource) — a
/// session should sync or broadcast, never both, and preferably against a
/// different server.
#[allow(async_fn_in_trait)] // futures are awaited in place, no Send bound needed
pub trait TxBroadcaster {
    /// Your transport error.
    type Error;

    /// Submit a serialized transaction to the network.
    async fn broadcast(&mut self, raw_tx: &[u8]) -> Result<(), Self::Error>;
}

/// A scheduled broadcast: plain data, safe to persist and restore.
///
/// The fields are public primitives on purpose — write them to whatever
/// storage your wallet already has. With the `serde` cargo feature enabled
/// the struct also derives `Serialize`/`Deserialize`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BroadcastPlan {
    /// The sampled delay, in seconds. Sampled once at scheduling time;
    /// resuming never re-samples (re-sampling on every restart would bias
    /// restarts toward short delays).
    pub delay_secs: u64,
    /// The profile it was sampled from.
    pub profile: Profile,
}

/// Why a resumed broadcast failed.
#[derive(Debug)]
pub enum BroadcastError<B, S> {
    /// Your transaction builder failed.
    Build(B),
    /// Your [`TxBroadcaster`] failed to send.
    Send(S),
}

impl<B: std::fmt::Display, S: std::fmt::Display> std::fmt::Display for BroadcastError<B, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BroadcastError::Build(e) => write!(f, "building the transaction failed: {e}"),
            BroadcastError::Send(e) => write!(f, "broadcasting failed: {e}"),
        }
    }
}

impl<B, S> std::error::Error for BroadcastError<B, S>
where
    B: std::fmt::Display + std::fmt::Debug,
    S: std::fmt::Display + std::fmt::Debug,
{
}

impl BroadcastPlan {
    /// The sampled delay.
    pub const fn delay(&self) -> Duration {
        Duration::from_secs(self.delay_secs)
    }

    /// How much of the delay is left after `elapsed` has already passed.
    pub fn remaining(&self, elapsed: Duration) -> Duration {
        self.delay().saturating_sub(elapsed)
    }

    /// Wait out the remainder of the delay, then build and send.
    ///
    /// Consumes the plan on purpose: firing is terminal, and a plan has no
    /// further meaning once its broadcast has gone out (the persisted copy
    /// is what outlives the process — clear it after firing, which the
    /// [`resume_pending`] helper does for you). The type is `Copy`, so
    /// consumption costs nothing.
    ///
    /// `elapsed` is how much time has passed since [`Scheduler::schedule`] —
    /// you persisted the scheduling moment, so you know. Freshly scheduled
    /// and never restarted? Pass `Duration::ZERO`.
    ///
    /// `build` runs only after the delay has fully elapsed — fetch the
    /// current tip *inside it* and set expiry with [`expiry_height`]. Its
    /// output goes straight to `broadcaster`.
    pub async fn resume<Bx, F, Fut, E>(
        self,
        elapsed: Duration,
        broadcaster: &mut Bx,
        build: F,
    ) -> Result<(), BroadcastError<E, Bx::Error>>
    where
        Bx: TxBroadcaster,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Vec<u8>, E>>,
    {
        let remaining = self.remaining(elapsed);
        // a min == max uniform delay is an exact, wasm-portable sleep; the
        // wrapped future — building included — is not polled early
        Delay::uniform(remaining, remaining)
            .run(async move {
                let raw_tx = build().await.map_err(BroadcastError::Build)?;
                broadcaster
                    .broadcast(&raw_tx)
                    .await
                    .map_err(BroadcastError::Send)
            })
            .await
    }
}

/// A [`BroadcastPlan`] coupled with the moment it was scheduled — the two
/// things a wallet must persist to resume after a restart. Plain data with
/// public fields; with the (default) `serde` feature it also derives
/// `Serialize`/`Deserialize`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StoredPlan {
    /// The sampled plan.
    pub plan: BroadcastPlan,
    /// When the plan was scheduled, in caller-supplied seconds (e.g. unix
    /// time). The crate never reads clocks — portable to wasm, and a wallet
    /// that misreports time degrades only its own anonymity.
    pub scheduled_at_secs: u64,
}

/// Your slot: persisting a pending broadcast plan in whatever storage your
/// wallet already has. At most one plan is stored at a time; saving replaces
/// any previous one.
///
/// Implement this once and the persist–restart–resume–clear lifecycle
/// becomes two calls: [`Scheduler::schedule_into`] at send time and
/// [`resume_pending`] at every startup.
#[allow(async_fn_in_trait)] // futures are awaited in place, no Send bound needed
pub trait PlanStore {
    /// Your storage error.
    type Error;

    /// Persist `plan`, replacing any previously stored one.
    async fn save(&mut self, plan: &StoredPlan) -> Result<(), Self::Error>;
    /// Load the pending plan, if one is stored.
    async fn load(&mut self) -> Result<Option<StoredPlan>, Self::Error>;
    /// Remove the stored plan (a no-op when none is stored).
    async fn clear(&mut self) -> Result<(), Self::Error>;
}

/// Why a store-integrated resumption failed.
#[derive(Debug)]
pub enum ResumePendingError<St, B, S> {
    /// The [`PlanStore`] failed to load, save, or clear.
    Store(St),
    /// The broadcast itself failed (building or sending).
    Broadcast(BroadcastError<B, S>),
}

impl<St: std::fmt::Display, B: std::fmt::Display, S: std::fmt::Display> std::fmt::Display
    for ResumePendingError<St, B, S>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResumePendingError::Store(e) => write!(f, "plan store failed: {e}"),
            ResumePendingError::Broadcast(e) => e.fmt(f),
        }
    }
}

impl<St, B, S> std::error::Error for ResumePendingError<St, B, S>
where
    St: std::fmt::Display + std::fmt::Debug,
    B: std::fmt::Display + std::fmt::Debug,
    S: std::fmt::Display + std::fmt::Debug,
{
}

impl Scheduler {
    /// Sample the delay — exactly once — and persist the plan through the
    /// store, stamped with `now_secs` (your clock, e.g. unix seconds).
    /// Follow up with [`resume_pending`], which also covers the happy path
    /// where the process never restarts.
    pub async fn schedule_into<S: PlanStore>(
        &mut self,
        store: &mut S,
        now_secs: u64,
    ) -> Result<StoredPlan, S::Error> {
        let stored = StoredPlan {
            plan: self.schedule(),
            scheduled_at_secs: now_secs,
        };
        store.save(&stored).await?;
        Ok(stored)
    }
}

/// Resume the pending broadcast, if the store holds one: wait out the
/// remaining delay (computed from `now_secs` against the stored scheduling
/// moment), build at fire time, hand the transaction to `broadcaster`, and
/// clear the store. Returns `Ok(false)` — with no waiting and no side
/// effects — when nothing is pending.
///
/// Call this at every wallet startup (and right after
/// [`Scheduler::schedule_into`]); a wallet that dies mid-delay then fires on
/// its next launch.
///
/// **Delivery is at-least-once, not exactly-once.** The store is cleared
/// only *after* the broadcast succeeds — clearing first would silently lose
/// the send if the process died in between, which is worse for a payment.
/// The flip side: a crash after the send but before the clear leaves the
/// plan pending, and the next startup will build and send *again*. Make
/// your builder idempotent with respect to the spend intent (lock the notes
/// you're spending, or have the builder check whether the intent already
/// reached the chain and return the same transaction).
pub async fn resume_pending<St, Bx, F, Fut, E>(
    store: &mut St,
    broadcaster: &mut Bx,
    now_secs: u64,
    build: F,
) -> Result<bool, ResumePendingError<St::Error, E, Bx::Error>>
where
    St: PlanStore,
    Bx: TxBroadcaster,
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Vec<u8>, E>>,
{
    let Some(stored) = store.load().await.map_err(ResumePendingError::Store)? else {
        return Ok(false);
    };
    let elapsed = Duration::from_secs(now_secs.saturating_sub(stored.scheduled_at_secs));
    stored
        .plan
        .resume(elapsed, broadcaster, build)
        .await
        .map_err(ResumePendingError::Broadcast)?;
    store.clear().await.map_err(ResumePendingError::Store)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockBroadcaster {
        sent: Vec<Vec<u8>>,
    }

    impl TxBroadcaster for MockBroadcaster {
        type Error = std::convert::Infallible;
        async fn broadcast(&mut self, raw_tx: &[u8]) -> Result<(), Self::Error> {
            self.sent.push(raw_tx.to_vec());
            Ok(())
        }
    }

    #[test]
    fn samples_respect_profile_bounds_and_mean() {
        let mut scheduler = Scheduler::standard().seed([1; 32]);
        let n = 10_000;
        let mut total = Duration::ZERO;
        for _ in 0..n {
            let plan = scheduler.schedule();
            assert!(plan.delay() <= blocks(576), "sample above the cap");
            total += plan.delay();
        }
        // the cap truncates the exponential, pulling the mean a few percent
        // below 144 blocks; anything in [0.85, 1.0] of nominal is healthy
        let mean = total / n;
        assert!(
            mean >= blocks(144).mul_f64(0.85) && mean <= blocks(144),
            "suspicious mean {mean:?}"
        );
    }

    #[test]
    fn fast_profile_is_proportionally_capped() {
        let mut scheduler = Scheduler::fast().seed([2; 32]);
        for _ in 0..1_000 {
            assert!(scheduler.schedule().delay() <= blocks(96));
        }
    }

    #[test]
    fn plans_are_independent_samples() {
        let mut scheduler = Scheduler::standard().seed([3; 32]);
        let first = scheduler.schedule();
        assert!(
            (0..100).any(|_| scheduler.schedule() != first),
            "every schedule drew the same delay"
        );
    }

    #[test]
    fn remaining_arithmetic() {
        let plan = BroadcastPlan {
            delay_secs: 1_000,
            profile: Profile::Standard,
        };
        assert_eq!(plan.remaining(Duration::ZERO), Duration::from_secs(1_000));
        assert_eq!(
            plan.remaining(Duration::from_secs(400)),
            Duration::from_secs(600)
        );
        // sleeping through the whole delay (and more) fires immediately
        assert_eq!(plan.remaining(Duration::from_secs(5_000)), Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn resume_waits_only_the_remainder_and_builds_at_fire_time() {
        let plan = BroadcastPlan {
            delay_secs: 1_000,
            profile: Profile::Standard,
        };
        let started = tokio::time::Instant::now();
        let mut broadcaster = MockBroadcaster { sent: Vec::new() };

        plan.resume(Duration::from_secs(400), &mut broadcaster, || async {
            // the builder observes the full remainder having elapsed: it ran
            // at fire time, not at schedule/resume time
            assert_eq!(started.elapsed(), Duration::from_secs(600));
            Ok::<_, std::convert::Infallible>(b"rawtx".to_vec())
        })
        .await
        .unwrap();

        assert_eq!(started.elapsed(), Duration::from_secs(600));
        assert_eq!(broadcaster.sent, vec![b"rawtx".to_vec()]);
    }

    #[tokio::test(start_paused = true)]
    async fn overslept_plan_fires_immediately() {
        let plan = BroadcastPlan {
            delay_secs: 100,
            profile: Profile::Fast,
        };
        let started = tokio::time::Instant::now();
        let mut broadcaster = MockBroadcaster { sent: Vec::new() };
        plan.resume(Duration::from_secs(4_000), &mut broadcaster, || async {
            Ok::<_, std::convert::Infallible>(vec![1])
        })
        .await
        .unwrap();
        assert_eq!(started.elapsed(), Duration::ZERO);
        assert_eq!(broadcaster.sent.len(), 1);
    }

    #[tokio::test]
    async fn build_failure_reaches_no_broadcaster() {
        let plan = BroadcastPlan {
            delay_secs: 0,
            profile: Profile::Standard,
        };
        let mut broadcaster = MockBroadcaster { sent: Vec::new() };
        let err = plan
            .resume(Duration::ZERO, &mut broadcaster, || async {
                Err::<Vec<u8>, _>("no spendable notes")
            })
            .await
            .unwrap_err();
        assert!(matches!(err, BroadcastError::Build("no spendable notes")));
        assert!(broadcaster.sent.is_empty());
    }

    #[test]
    fn refresh_decision_boundary() {
        let tip = 3_000_000;
        assert!(!needs_refresh_sync(tip, tip));
        assert!(!needs_refresh_sync(tip - ZIP318_ANCHOR_RETENTION, tip));
        assert!(needs_refresh_sync(tip - ZIP318_ANCHOR_RETENTION - 1, tip));
    }

    #[test]
    fn conversions() {
        assert_eq!(blocks(1), Duration::from_secs(75));
        assert_eq!(blocks(144), Duration::from_secs(10_800)); // ~3 h
        assert_eq!(expiry_height(3_000_000), 3_000_040);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip() {
        let plan = Scheduler::standard().seed([4; 32]).schedule();
        let json = serde_json::to_string(&plan).unwrap();
        let back: BroadcastPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(plan, back);
    }

    #[derive(Default)]
    struct MemStore {
        stored: Option<StoredPlan>,
    }

    impl PlanStore for MemStore {
        type Error = std::convert::Infallible;
        async fn save(&mut self, plan: &StoredPlan) -> Result<(), Self::Error> {
            self.stored = Some(*plan);
            Ok(())
        }
        async fn load(&mut self) -> Result<Option<StoredPlan>, Self::Error> {
            Ok(self.stored)
        }
        async fn clear(&mut self) -> Result<(), Self::Error> {
            self.stored = None;
            Ok(())
        }
    }

    #[tokio::test]
    async fn store_lifecycle_schedules_resumes_and_clears() {
        let scheduled_at = 1_700_000_000u64;
        let mut store = MemStore::default();
        let stored = Scheduler::standard()
            .seed([21; 32])
            .schedule_into(&mut store, scheduled_at)
            .await
            .unwrap();
        assert_eq!(
            store.stored,
            Some(stored),
            "plan persisted at schedule time"
        );

        // "restart": all in-memory state gone, only the store remains
        let mut broadcaster = MockBroadcaster { sent: Vec::new() };
        let fired = resume_pending(
            &mut store,
            &mut broadcaster,
            scheduled_at + stored.plan.delay_secs + 1,
            || async { Ok::<_, std::convert::Infallible>(vec![7]) },
        )
        .await
        .unwrap();

        assert!(fired);
        assert_eq!(broadcaster.sent, vec![vec![7]]);
        assert_eq!(store.stored, None, "a fired plan must be cleared");
    }

    #[tokio::test]
    async fn resume_pending_without_plan_is_a_noop() {
        let mut store = MemStore::default();
        let mut broadcaster = MockBroadcaster { sent: Vec::new() };
        let fired = resume_pending(&mut store, &mut broadcaster, 1_700_000_000, || async {
            Ok::<_, std::convert::Infallible>(vec![1])
        })
        .await
        .unwrap();
        assert!(!fired);
        assert!(broadcaster.sent.is_empty(), "no plan, no broadcast");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn stored_plan_serde_round_trip() {
        let stored = StoredPlan {
            plan: Scheduler::fast().seed([22; 32]).schedule(),
            scheduled_at_secs: 1_700_000_000,
        };
        let json = serde_json::to_string(&stored).unwrap();
        let back: StoredPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(stored, back);
    }
}
