// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Delay a broadcast by a random duration.
//!
//! Broadcast timing is a leak the destination observes directly: a wallet
//! that broadcasts the moment it reaches chain tip is trivially correlatable
//! with its own sync activity. Decorrelate by scheduling the broadcast
//! through a randomized timer.
//!
//! Kept app-level (out of scope for this crate): never send the broadcast
//! over the sync transport or session, and consider destination splitting —
//! sync from one server, broadcast through another.
//!
//! Run with: `cargo run -p nym-swizzle --example delay_broadcast`

use std::time::{Duration, Instant};

use nym_swizzle::Delay;

async fn broadcast_tx(tx: &str) -> String {
    // stand-in for handing the transaction to the network
    format!("broadcast: {tx}")
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // delays sampled uniformly from [0, 3] seconds; use larger bounds in a
    // real wallet
    let mut scheduler = Delay::uniform(Duration::ZERO, Duration::from_secs(3));

    let started = Instant::now();
    let result = scheduler
        .run(async move { broadcast_tx("tx-deadbeef").await })
        .await;

    println!("{result}");
    println!("observable send happened after {:?}", started.elapsed());
    println!("(the async block was not polled until its scheduled time)");
}
