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
//!    the wire: a grid-aligned range, fetched as overlapping shuffled chunks
//!    through *your* transport (the `BlockSource` implementation below is the
//!    slot where your own lightwalletd client goes).
//! 2. **Broadcast, decoupled.** We schedule a send, persist the plan to a
//!    file, throw everything away, restore it — as if the wallet was killed
//!    and reopened hours later — and resume. The transaction is *built* only
//!    at fire time, with an expiry from a freshly fetched tip, and lands in a
//!    mock broadcaster (sending for real needs a funded seed; the mock prints
//!    what it would have sent).
//!
//! ```sh
//! cargo run --release -p nym-swizzle-zcash --example wallet_sync
//!
//! # knobs (defaults shown)
//! ZEC_SERVER=https://zec.rocks:443 ZEC_GAP=400 \
//!     cargo run --release -p nym-swizzle-zcash --example wallet_sync
//! ```
//!
//! This talks to a real public server — the defaults are deliberately gentle
//! (one grid cell, usually ~1152 compact blocks).

#[path = "support/lightwalletd.rs"]
mod lightwalletd;

use std::time::Duration;

use nym_swizzle_zcash::broadcast::{
    expiry_height, needs_refresh_sync, BroadcastPlan, Profile, Scheduler, TxBroadcaster,
};
use nym_swizzle_zcash::grid::{quantize, Disposition, QueuedRange};
use nym_swizzle_zcash::sync::{BlockSource, SyncSession};

use crate::lightwalletd::{CompactBlock, Lightwalletd};

const DEFAULT_SERVER: &str = "https://zec.rocks:443";
const DEFAULT_GAP: u64 = 400;

type BoxError = Box<dyn std::error::Error>;

fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
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

/// A `BroadcastPlan` is plain data. This example persists it as two lines of
/// text to show there's nothing up its sleeve — with the crate's `serde`
/// feature you'd `serde_json::to_string(&plan)` instead. A real wallet also
/// stores *when* it scheduled, to compute elapsed time after a restart.
fn save_plan(path: &std::path::Path, plan: &BroadcastPlan) -> Result<(), BoxError> {
    let profile = match plan.profile {
        Profile::Standard => "standard",
        Profile::Fast => "fast",
    };
    std::fs::write(
        path,
        format!("delay_secs={}\nprofile={profile}\n", plan.delay_secs),
    )?;
    Ok(())
}

fn load_plan(path: &std::path::Path) -> Result<BroadcastPlan, BoxError> {
    let text = std::fs::read_to_string(path)?;
    let mut delay_secs = None;
    let mut profile = None;
    for line in text.lines() {
        match line.split_once('=') {
            Some(("delay_secs", v)) => delay_secs = Some(v.parse()?),
            Some(("profile", "standard")) => profile = Some(Profile::Standard),
            Some(("profile", "fast")) => profile = Some(Profile::Fast),
            _ => return Err(format!("unrecognised plan line: {line}").into()),
        }
    }
    Ok(BroadcastPlan {
        delay_secs: delay_secs.ok_or("plan missing delay_secs")?,
        profile: profile.ok_or("plan missing profile")?,
    })
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let server: String = env_or("ZEC_SERVER", DEFAULT_SERVER.to_string());
    let gap: u64 = env_or("ZEC_GAP", DEFAULT_GAP);
    assert!(gap > 0, "ZEC_GAP must be positive");

    println!("connecting to {server} ...");
    let mut client = Lightwalletd::connect(server).await?;
    let tip = client.tip().await?;

    // ---- 1. sync: quantized range, swizzled chunks --------------------

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
    println!("   this wallet sends chunks covering: {es}..{ee}");
    println!(
        "      grid spacing {}; start is a grid multiple ({es} = {} x {}), so every",
        quantized.spacing(),
        es / quantized.spacing(),
        quantized.spacing()
    );
    println!("      wallet resuming anywhere in this cell emits the same boundaries.\n");
    println!("   requests on the wire (shuffled, overlapping — checkable below):");

    let mut source = LoggingSource {
        client: client.clone(),
        requests: Vec::new(),
    };
    let mut counts = std::collections::BTreeMap::<Disposition, u64>::new();
    let outcome = SyncSession::new()
        .fetch(
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

    // ---- 2. broadcast: schedule, persist, restart, resume --------------

    println!("\nscheduling a broadcast (decoupled from the sync above):");
    let plan = Scheduler::standard().schedule();
    println!(
        "   sampled delay: {:.1} h (exponential, mean ~3 h, capped at ~12 h — ZIP 318's",
        plan.delay().as_secs_f64() / 3600.0
    );
    println!("   transfer-scheduling parameters, so sends pool with migration traffic)");

    let path = std::env::temp_dir().join("nym-swizzle-zcash-example-plan.txt");
    save_plan(&path, &plan)?;
    println!(
        "   plan persisted to {} — plain data, no live state",
        path.display()
    );

    println!("\n   ... pretend the wallet was killed and reopened after the delay ...\n");

    let restored = load_plan(&path)?;
    std::fs::remove_file(&path).ok();

    // do we need to sync again before sending? only if the anchors aged out
    let fresh_tip = client.tip().await?;
    println!(
        "   refresh sync needed? {} (last synced {tip}, tip now {fresh_tip}, anchors live ~2 days)",
        if needs_refresh_sync(tip, fresh_tip) {
            "yes — sync on its own session first"
        } else {
            "no"
        }
    );

    // elapsed >= the sampled delay, so this fires immediately; a wallet that
    // wakes up early just sleeps the remainder
    let elapsed = restored.delay() + Duration::from_secs(1);
    restored
        .resume(elapsed, &mut MockBroadcaster, || async {
            // built at FIRE time: expiry comes from a tip fetched now, not
            // from anything remembered at scheduling time
            let tip_at_fire = client.tip().await?;
            let expiry = expiry_height(tip_at_fire);
            println!("      building now: tip {tip_at_fire}, expiry {expiry} (tip + 40)");
            Ok::<_, tonic::Status>(format!("demo-tx-expiry-{expiry}").into_bytes())
        })
        .await?;

    println!(
        "\ndone. things to poke at: rerun and watch the chunk boundaries change while the\n\
         {es}..{ee} union stays put; move ZEC_GAP around a grid boundary; and run the\n\
         opt-in live tests (cargo test -p nym-swizzle-zcash -- --ignored) for the\n\
         reorg-detection path against real chain data."
    );

    Ok(())
}
