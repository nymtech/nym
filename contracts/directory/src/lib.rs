// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

/// CosmWasm entry points (`instantiate`, `execute`, `query`, `migrate`).
pub mod contract;

/// One-shot data migrations executed by the `migrate` entry point.
pub mod queued_migrations;

/// `cw-storage-plus` definitions
pub mod storage;

/// Read-only query handlers backing [`contract::query`].
mod queries;

/// Test-only helpers — always compiled for this crate's own unit tests via
/// `cfg(test)`; downstream crates can pull them in for their own test
/// harnesses by enabling the `testable-directory-contract` feature.
#[cfg(any(test, feature = "testable-directory-contract"))]
pub mod testing;

/// State-mutating execute handlers backing [`contract::execute`].
mod transactions;
