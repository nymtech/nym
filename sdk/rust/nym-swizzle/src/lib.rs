// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! # nym-swizzle
//!
//! Application-layer traffic-shape obfuscation for privacy-preserving apps —
//! primarily wallets, or anything that fetches sequential, index-addressed data
//! (blocks, notes, checkpoints) or broadcasts at meaningful moments.
//!
//! ## Threat model
//!
//! A mixnet (or any transport anonymity layer) hides *who* is talking; it does
//! not hide *what an application's query pattern says about it*:
//!
//! - **Timing correlation.** The destination observes wall-clock arrival. A
//!   wallet that broadcasts a transaction immediately after reaching chain tip
//!   is trivially correlatable with its own sync activity. Use [`delay`] to
//!   decorrelate observable actions from the events that trigger them.
//! - **Index / content correlation.** A light client that requests exactly
//!   blocks `4_120_000..4_120_010` reveals its resume point, and the start
//!   height acts as a *linking key across sessions*: today's start is
//!   yesterday's end. Use [`range`] to fetch via overlapping, shuffled chunks,
//!   with a randomized start overlap and/or checkpoint snapping so starts stop
//!   being exact pointers to previous ends.
//!
//! ## What stays your responsibility
//!
//! - **Transport and destination splitting** (sync from one server, broadcast
//!   through another; never broadcast over the sync session) — app-level.
//! - **Range widening for interest-masking**: this crate never extends the
//!   *end* of a range and cannot know which indexes exist (chain tip, array
//!   bounds). If you want to mask *which* sub-range you care about, widen the
//!   requested range yourself; the crate obfuscates coverage of whatever range
//!   it is given. The one sanctioned outward extension is the *downward* start
//!   overlap, where earlier indexes always exist.
//! - **Deduplication**: overlapping chunks deliberately re-fetch data.
//!   Index-addressed data is idempotent; dedup is yours.
//!
//! ## Tuning caveat
//!
//! Wider overlaps and checkpoint spacing buy a larger anonymity set at the
//! cost of re-downloaded data. There are **no settled numbers** for this
//! trade-off; the defaults here are conservative percentage-of-range
//! derivations, exposed as knobs, not validated recommendations.
//!
//! ## Examples
//!
//! Delay a broadcast by a random duration:
//!
//! ```no_run
//! # async fn broadcast_tx() {}
//! # async fn example() {
//! use std::time::Duration;
//!
//! let mut s = nym_swizzle::delay::Delay::uniform(Duration::ZERO, Duration::from_secs(10));
//! let result = s.run(async move { broadcast_tx().await }).await;
//! # }
//! ```
//!
//! Fetch an index range via overlapping, shuffled chunks:
//!
//! ```no_run
//! # async fn get_block(_s: u64, _e: u64) {}
//! # async fn example() {
//! let mut s = nym_swizzle::range::Range::new(0, 1000).plan();
//! while let Some((start, end)) = s.next() {
//!     get_block(start, end).await;
//! }
//! # }
//! ```
//!
//! ## Wasm
//!
//! Every non-dev dependency compiles for `wasm32-unknown-unknown`; the crate
//! is designed to be wrapped, unmodified, by a `wasm-pack` wrapper crate. On
//! wasm targets, timing uses [`wasmtimer`] and randomness reaches the browser
//! through `getrandom`'s `wasm_js` backend (enable it with
//! `RUSTFLAGS='--cfg getrandom_backend="wasm_js"'`).

#![warn(missing_docs)]

pub mod delay;
pub mod range;
pub mod rng;

pub(crate) mod timer;

pub use delay::Delay;
pub use range::{ChunkPlan, Range, Snap};
pub use rng::Sampling;
