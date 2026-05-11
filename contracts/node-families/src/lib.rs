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
/// Test-only helpers — only compiled when running the contract's unit tests.
#[cfg(test)]
pub mod testing;
/// State-mutating execute handlers backing [`contract::execute`].
mod transactions;
