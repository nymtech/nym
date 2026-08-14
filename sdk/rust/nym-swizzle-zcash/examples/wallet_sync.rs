// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! What your lightwalletd sees when a wallet syncs the private way — live,
//! against a real server, so you can check every claim yourself.
//!
//! Two things happen here, both with real chain data and neither needing a
//! wallet seed:
//!
//! 1. **Sync.** We simulate a wallet resuming a few hundred blocks behind the
//!    tip, and print the request a naive wallet would send (its exact resume
//!    point — a linking key across sessions) next to what actually goes on
//!    the wire: a grid-aligned range, fetched as a deterministic ascending
//!    sequence of grid-cell requests through *your* transport (the
//!    `BlockSource` implementation below is the slot where your own
//!    lightwalletd client goes). Determinism is deliberate: every wallet
//!    resuming in the same cell says exactly the same thing.
//! 2. **Broadcast, decoupled.** We schedule a send through the `PlanStore`
//!    slot (a JSON file here; your wallet's database in real life), print
//!    exactly what the plan commits to (profile, sampled delay in blocks and
//!    hours, projected fire height, the literal bytes persisted), then play
//!    out five wake-ups a couple of seconds apart: each round the wallet is
//!    "killed and reopened", loads the plan from the store, sees on its
//!    clock that the delay hasn't elapsed, and goes back to sleep — making
//!    **no network calls** until fire time, exactly as a real wallet should.
//!    The hours of delay are compressed into seconds (the clock, and the
//!    ~one-block-per-75-s chain drift it implies, are simulated); on the
//!    final round `resume_pending` builds the transaction at fire time with
//!    an expiry from a freshly fetched **real** tip, hands it to a mock
//!    broadcaster (sending for real needs a funded seed), and clears the
//!    store.
//!
//! ```sh
//! cargo run --release -p nym-swizzle-zcash --example wallet_sync
//!
//! # knobs (defaults shown)
//! ZEC_SERVER=https://zec.rocks:443 ZEC_GAP=400 ZEC_ROUNDS=5 ZEC_ROUND_SECS=2 \
//!     cargo run --release -p nym-swizzle-zcash --example wallet_sync
//! ```
//!
//! This talks to a real public server — the defaults are deliberately gentle
//! (one grid cell, usually ~1152 compact blocks, and one `GetLatestBlock` at
//! fire time).

#[path = "support/lightwalletd.rs"]
mod lightwalletd;

use std::time::Duration;

use nym_swizzle_zcash::broadcast::{
    expiry_height, needs_refresh_sync, resume_pending, PlanStore, Scheduler, StoredPlan,
    TxBroadcaster, TARGET_BLOCK_TIME,
};
use nym_swizzle_zcash::grid::{quantize, Disposition, QueuedRange};
use nym_swizzle_zcash::sync::{self, BlockSource};

use crate::lightwalletd::{CompactBlock, Lightwalletd};

const DEFAULT_SERVER: &str = "https://zec.rocks:443";
const DEFAULT_GAP: u64 = 400;
const DEFAULT_ROUNDS: u32 = 5;
/// Real seconds between wake-ups — pacing for readability only; the wallet's
/// clock (and the chain drift it implies) is simulated so the demo runs in
/// seconds instead of hours.
const DEFAULT_ROUND_SECS: u64 = 2;

type BoxError = Box<dyn std::error::Error>;

/// Absent knob → documented default; *malformed* knob → loud abort. A demo
/// that silently substitutes a default for a typo teaches the wrong habit.
fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T
where
    T::Err: std::fmt::Display,
{
    match std::env::var(key) {
        Ok(value) => value
            .parse()
            .unwrap_or_else(|e| panic!("{key}={value} is not a valid value: {e}")),
        Err(_) => default,
    }
}

/// Your slot: the crate decides *which* ranges to request, your type does
/// the requesting. This one wraps the gRPC client and logs every request so
/// you can see exactly what the server sees.
struct LoggingSource {
    client: Lightwalletd,
    requests: Vec<(u64, u64)>,
}

impl BlockSource for LoggingSource {
    type Block = CompactBlock;
    type Error = tonic::Status;

    async fn block_range(
        &mut self,
        start: u64,
        end: u64,
    ) -> Result<Vec<(u64, CompactBlock)>, tonic::Status> {
        self.requests.push((start, end));
        println!(
            "      -> GetBlockRange({start}..{end})  [{} blocks]",
            end - start
        );
        let blocks = self.client.block_range(start, end).await?;
        Ok(blocks.into_iter().map(|b| (b.height, b)).collect())
    }
}

/// Your other slot — sending. Deliberately a separate trait from
/// `BlockSource`: a session should sync or broadcast, never both. This mock
/// stands in for a client pointed at a *different* server than the sync one.
struct MockBroadcaster;

impl TxBroadcaster for MockBroadcaster {
    type Error = std::convert::Infallible;

    async fn broadcast(&mut self, raw_tx: &[u8]) -> Result<(), Self::Error> {
        println!(
            "      -> SendTransaction({} bytes) — mocked; a real send needs a funded seed",
            raw_tx.len()
        );
        Ok(())
    }
}

/// Your third slot — persistence. One JSON file via the crate's (default)
/// `serde` feature; a real wallet would point this at its existing database.
struct FilePlanStore {
    path: std::path::PathBuf,
}

impl PlanStore for FilePlanStore {
    type Error = BoxError;

    async fn save(&mut self, plan: &StoredPlan) -> Result<(), BoxError> {
        std::fs::write(&self.path, serde_json::to_vec_pretty(plan)?)?;
        Ok(())
    }

    async fn load(&mut self) -> Result<Option<StoredPlan>, BoxError> {
        match std::fs::read(&self.path) {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn clear(&mut self) -> Result<(), BoxError> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let server: String = env_or("ZEC_SERVER", DEFAULT_SERVER.to_string());
    let gap: u64 = env_or("ZEC_GAP", DEFAULT_GAP);

    println!("connecting to {server} ...");
    let mut client = Lightwalletd::connect(server).await?;
    let tip = client.tip().await?;
    assert!(
        gap > 0 && gap < tip,
        "ZEC_GAP={gap} must be between 1 and the chain tip ({tip})"
    );

    // ---- 1. sync: quantized range, deterministic requests --------------

    let resume = tip - gap;
    let range = QueuedRange::catch_up(resume, tip);
    let quantized = quantize(range, tip);
    let (es, ee) = quantized.emitted();

    println!("\nchain tip {tip}; simulating a wallet that last synced at {resume}\n");
    println!(
        "   naive wallet would send:  GetBlockRange({resume}..{})",
        tip + 1
    );
    println!("      the start height IS the wallet's previous session — a linking key\n");
    println!("   this wallet requests exactly: {es}..{ee}");
    println!(
        "      grid spacing {}; start is a grid multiple ({es} = {} x {}), so every",
        quantized.spacing(),
        es / quantized.spacing(),
        quantized.spacing()
    );
    println!("      wallet resuming anywhere in this cell says exactly the same thing.\n");
    println!("   requests on the wire (deterministic: ascending 1152-block grid cells,");
    println!("   no randomness — variation inside a collision set would only stand out):");

    let mut source = LoggingSource {
        client: client.clone(),
        requests: Vec::new(),
    };
    let mut counts = std::collections::BTreeMap::<Disposition, u64>::new();
    let outcome = sync::fetch(
        &mut source,
        range,
        tip,
        |_, _, disposition| *counts.entry(disposition).or_default() += 1,
        |height, _block| {
            // your wallet compares its stored hash for `height` here; this
            // demo wallet has no database, so it accepts what it sees
            let _ = height;
            true
        },
    )
    .await?;

    let n = |d: Disposition| counts.get(&d).copied().unwrap_or(0);
    let wanted = n(Disposition::Requested);
    let cover: u64 = counts
        .iter()
        .filter(|(d, _)| **d != Disposition::Requested)
        .map(|(_, c)| c)
        .sum();
    println!("\n   outcome: {outcome:?} (commit scan results only on Committed)");
    println!(
        "   {} blocks streamed: {wanted} wanted, {} verify-window, {} cover below, {} cover above",
        wanted + cover,
        n(Disposition::VerifyWindow),
        n(Disposition::CoverBelow),
        n(Disposition::CoverAbove),
    );
    println!(
        "   cover overhead: {:.0}% — the bandwidth price of being indistinguishable from \
         every other wallet in the cell",
        cover as f64 / wanted as f64 * 100.0,
    );

    // check the boundary claims hold for what was actually sent
    let min_start = source
        .requests
        .iter()
        .map(|r| r.0)
        .min()
        .expect("requests logged");
    let max_end = source
        .requests
        .iter()
        .map(|r| r.1)
        .max()
        .expect("requests logged");
    assert_eq!(
        (min_start, max_end),
        (es, ee),
        "wire union must equal the emitted range"
    );
    assert_eq!(
        min_start % quantized.spacing(),
        0,
        "start must be grid-aligned"
    );

    // ---- 2. broadcast: schedule, persist, wake up repeatedly, resume ----

    let rounds: u32 = env_or("ZEC_ROUNDS", DEFAULT_ROUNDS);
    let round_wait: u64 = env_or("ZEC_ROUND_SECS", DEFAULT_ROUND_SECS);
    assert!(rounds >= 1, "ZEC_ROUNDS must be at least 1");

    println!("\nscheduling a broadcast (decoupled from the sync above):");
    // the demo's clock: hours are compressed into wake-up rounds, so "now"
    // is simulated; a real wallet passes unix time in both places
    let scheduled_at = 1u64;
    let mut store = FilePlanStore {
        path: std::env::temp_dir().join("nym-swizzle-zcash-example-plan.json"),
    };
    let stored = Scheduler::standard()
        .schedule_into(&mut store, scheduled_at)
        .await?;
    let delay_blocks = stored.plan.delay_secs / TARGET_BLOCK_TIME.as_secs();
    println!("   profile: standard — exponential delay, mean 144 blocks (~3 h), samples above");
    println!("   576 blocks (~12 h) re-drawn; ZIP 318's transfer-scheduling parameters, so");
    println!("   this send pools with everyone's migration traffic");
    println!(
        "   sampled delay: {:.1} h = {delay_blocks} blocks — the transaction will be built and",
        stored.plan.delay().as_secs_f64() / 3600.0
    );
    println!(
        "   sent around height {} (scheduled at {tip}), from a tip fetched at fire time",
        tip + delay_blocks
    );

    println!(
        "\n   the whole plan is three integers, persisted through the PlanStore slot to {}:",
        store.path.display()
    );
    for line in std::fs::read_to_string(&store.path)?.lines() {
        println!("      | {line}");
    }
    println!(
        "\n   ... the demo now compresses those {:.1} h into {rounds} wake-ups, {round_wait} s apart ...",
        stored.plan.delay().as_secs_f64() / 3600.0
    );

    // The wallet now gets killed and reopened over and over while the delay
    // runs down. Each round below is one such wake-up: load the plan from
    // the store, see on the (simulated) clock that it isn't time yet, go
    // back to sleep. No network traffic on early wake-ups — a real wallet
    // checks nothing but its own clock until fire time — so the chain
    // heights shown are the drift the wallet would *expect* at ~one block
    // per 75 s; the real tip is fetched exactly once, when the transaction
    // is built.
    for round in 1..=rounds {
        if round_wait > 0 {
            tokio::time::sleep(Duration::from_secs(round_wait)).await;
        }

        // "reopened": nothing in memory, only the store
        let pending = store.load().await?.expect("plan is pending until fired");
        let elapsed = pending
            .plan
            .delay()
            .mul_f64(f64::from(round) / f64::from(rounds));
        let now_secs = pending.scheduled_at_secs + elapsed.as_secs();
        let expected_tip = tip + elapsed.as_secs() / TARGET_BLOCK_TIME.as_secs();

        if round < rounds {
            println!(
                "\n   wake-up {round}/{rounds}: loaded plan from the store; no network calls — \
                 the chain should be\n      near {expected_tip} (+{} blocks while asleep); {:.1} h \
                 of {:.1} h elapsed, {:.1} h left —\n      not time yet, back to sleep",
                expected_tip - tip,
                elapsed.as_secs_f64() / 3600.0,
                pending.plan.delay().as_secs_f64() / 3600.0,
                pending.plan.remaining(elapsed).as_secs_f64() / 3600.0,
            );
            continue;
        }

        println!(
            "\n   wake-up {round}/{rounds}: loaded plan from the store — the delay has elapsed, firing:"
        );

        // fire time: NOW the wallet talks to the network again. Do we need a
        // refresh sync first? Only if the anchors aged out while we slept.
        let tip_now = client.tip().await?;
        println!(
            "      refresh sync needed? {} (last synced {tip}, tip now {tip_now}, anchors live ~2 days)",
            if needs_refresh_sync(tip, tip_now) {
                "yes — sync on its own session first"
            } else {
                "no"
            }
        );

        // one call covers load → wait the remainder (zero here: the clock
        // says the delay fully elapsed) → build at fire time → send → clear
        let fired = resume_pending(&mut store, &mut MockBroadcaster, now_secs + 1, || async {
            // built at FIRE time: expiry comes from a tip fetched now, not
            // from anything remembered at scheduling time
            let tip_at_fire = client.tip().await?;
            let expiry = expiry_height(tip_at_fire);
            println!("      building now: tip {tip_at_fire}, expiry {expiry} (tip + 40)");
            Ok::<_, tonic::Status>(format!("demo-tx-expiry-{expiry}").into_bytes())
        })
        .await
        .map_err(|e| format!("broadcast failed: {e}"))?;
        assert!(fired, "a plan was pending, so resume_pending must fire");
        assert!(
            store.load().await?.is_none(),
            "the fired plan must be cleared from the store"
        );
    }

    println!(
        "\ndone. things to poke at: rerun and watch the {es}..{ee} requests repeat\n\
         byte-identically — any wallet resuming anywhere in this cell sends exactly\n\
         these; move ZEC_GAP around a grid boundary; and run the opt-in live tests\n\
         (NYM_SWIZZLE_ZCASH_LIVE_TESTS=1 cargo test -p nym-swizzle-zcash) for the\n\
         reorg-detection path\n\
         against real chain data."
    );

    Ok(())
}
