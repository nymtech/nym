// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! # nym-swizzle-zcash
//!
//! Baseline privacy hygiene for Zcash light clients, built on
//! [`nym_swizzle`]. Even with perfect transport anonymity, a lightwalletd
//! learns two things from request *content and timing*:
//!
//! - **Resume-point chaining.** A wallet that resumes syncing at exactly
//!   `previous end + 1` names one specific block per session, and each
//!   session's start links it to the previous one. The server can track a
//!   wallet by height alone — no IP address needed.
//! - **Sync-then-send.** A wallet that syncs and immediately broadcasts lets
//!   the server attribute the transaction to the sync session seconds before
//!   it — and, via chaining, to the wallet's whole history.
//!
//! This crate packages two countermeasures as a thin policy layer:
//!
//! - [`grid`] — **quantized sync ranges**: every emitted `GetBlockRange`
//!   is widened to a network-wide grid, so all wallets resuming in the same
//!   grid cell send byte-identical requests (anonymity by collision).
//!   [`sync`] puts those ranges on the wire deterministically — grid-aligned
//!   cells, ascending, no randomness, because variation within a collision
//!   set only makes a wallet more distinguishable, not less.
//! - [`broadcast`] — **decoupled broadcast scheduling**: transactions are
//!   sent after a randomized multi-hour delay calibrated to blend with
//!   network-wide traffic, from a schedule that survives wallet restarts.
//!
//! ## What stays your responsibility
//!
//! The crate performs **no network I/O**. You implement two small traits —
//! [`sync::BlockSource`] for fetching compact blocks and
//! [`broadcast::TxBroadcaster`] for sending raw transactions — with whatever
//! transport you already use. Keeping them separate is deliberate: a session
//! should sync or broadcast, never both, and ideally against different
//! servers.
//!
//! ## Wasm
//!
//! Every non-dev dependency compiles for `wasm32-unknown-unknown`; the
//! gRPC-flavoured code lives only in the examples and network tests.

#![warn(missing_docs)]

pub mod broadcast;
pub mod grid;
pub mod sync;

pub use broadcast::{
    blocks, expiry_height, needs_refresh_sync, BroadcastError, BroadcastPlan, PlanStore, Profile,
    ResumePendingError, Scheduler, StoredPlan, TxBroadcaster, TARGET_BLOCK_TIME,
};
pub use grid::{classify, quantize, Disposition, Quantized, QueuedRange, RangeKind};
pub use sync::{BlockSource, SyncError, SyncOutcome};
