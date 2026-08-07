// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Fetch a block range via overlapping, shuffled chunks.
//!
//! Requesting exactly the blocks you need tells the serving node your resume
//! point, and start heights chain sessions together (today's start is
//! yesterday's end). This example resumes a sync at height 4_120_137 and
//! obfuscates the start two ways at once:
//!
//! - checkpoint snapping (spacing 1000): every client resuming in the same
//!   interval emits the identical start 4_120_000
//! - start jitter: the start additionally moves down a random whole number of
//!   checkpoint intervals
//!
//! The chunks overlap wastefully by design; blocks are idempotent, so
//! re-fetched blocks are simply deduplicated by the caller.
//!
//! Run with: `cargo run -p nym-swizzle --example fetch_blocks_overlapping`

use futures::StreamExt;
use nym_swizzle::{Range, Snap};

async fn get_blocks(start: u64, end: u64) -> usize {
    // stand-in for a lightwalletd-style compact-block range fetch
    println!("  get_blocks({start}, {end})  [{} blocks]", end - start);
    (end - start) as usize
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let true_resume_point = 4_120_137;
    let tip = 4_121_500;

    let plan = Range::new(true_resume_point, tip)
        .chunk_size(50..=200)
        .overlap(5..=40)
        .snap_start(Snap::Spacing(1000))
        .start_jitter(2500)
        .plan();

    println!(
        "true resume point {true_resume_point}, emitted start {} (on-grid, deniable)",
        plan.start()
    );
    println!(
        "{} overlapping chunks, executed 4 at a time in random order:",
        plan.len()
    );

    // `stream_concurrent` yields each chunk's result as it completes, so the
    // tally needs no shared state threaded through the closure
    let total_fetched: usize = plan
        .stream_concurrent(4, get_blocks)
        .fold(0, |acc, fetched| async move { acc + fetched })
        .await;

    let total_needed = (tip - true_resume_point) as usize;
    println!(
        "fetched {total_fetched} blocks to cover {total_needed} \
         ({}% deliberate waste)",
        (total_fetched - total_needed) * 100 / total_needed
    );
}
