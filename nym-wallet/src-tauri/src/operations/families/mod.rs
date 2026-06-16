// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Tauri command layer for the `node-families-contract`.
//!
//! Mirrors `operations/mixnet/`: each `#[tauri::command]` acquires the active
//! account's signing client from the wallet [`WalletState`](crate::state::WalletState)
//! and delegates to the existing `validator-client` `NodeFamilies{Signing,Query}Client`
//! traits, returning the shapes the `src/requests/families.ts` bindings expect.
//!
//! Split into `execute` (state-changing txs) and `query` (read-only) to match the
//! rest of the wallet (design D1).

pub mod execute;
pub mod query;
