// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! CosmWasm contract that manages "node families" — owner-led groupings of
//! Nym nodes — including their members, pending invitations, and historical
//! records of past members and rejected/revoked invitations.
//!
//! The shared message and type surface lives in
//! [`node_families_contract_common`]; this crate contains only the on-chain logic
//! and storage layout.

/// CosmWasm entry points (`instantiate`, `execute`, `query`, `migrate`).
pub mod contract;
/// One-shot data migrations executed by the `migrate` entry point.
pub mod queued_migrations;
/// `cw-storage-plus` definitions: typed maps, items and secondary indexes.
pub mod storage;

mod helpers;
/// Read-only query handlers backing [`contract::query`].
mod queries;
/// Test-only helpers — compiled when the `testable-node-families-contract`
/// feature is on (the contract's own tests activate it via the dev-dep trick).
/// Downstream crates can pull it in for their own test harnesses by depending
/// on this crate with the same feature enabled.
#[cfg(feature = "testable-node-families-contract")]
pub mod testing;
/// State-mutating execute handlers backing [`contract::execute`].
mod transactions;
