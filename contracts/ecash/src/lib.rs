// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! On-chain anchor of the ticketbook credential pipeline.
//!
//! Clients escrow funds via `ExecuteMsg::DepositTicketBookFunds`, which mints a
//! sequential `deposit_id` and persists the depositor-claimed ed25519 identity
//! key for off-chain nym-api signers to read at blind-sign time. The contract
//! does **not** verify control of the ed25519 key - that proof is enforced
//! off-chain by `nym-api/src/ecash/deposit.rs::validate_deposit`.
//!
//! See `openspec/specs/ecash-contract/spec.md` for the normative interface.

mod constants;

/// CosmWasm entry points (`instantiate`, `execute`, `query`, `migrate`, `reply`).
pub mod contract;

mod deposit;
mod deposit_stats;
mod helpers;

/// One-shot data migrations executed by the `migrate` entry point.
pub mod queued_migrations;

/// Read-only query handlers backing [`contract::query`].
mod queries;

/// `cw-storage-plus` definitions
pub mod storage;

/// State-mutating execute and reply handlers backing [`contract::execute`]
/// and [`contract::reply`].
mod transactions;

#[cfg(test)]
pub mod multitest;
#[cfg(test)]
mod wire_compat;
