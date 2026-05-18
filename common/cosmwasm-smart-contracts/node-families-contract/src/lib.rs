// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Common types, messages, errors and storage-key constants shared between the
//! node families contract and any off-chain client.
//!
//! Keeping these in a separate crate allows clients to depend on the contract's
//! public surface without pulling in `cw-storage-plus` and other on-chain-only
//! dependencies.

/// Storage-key string constants. See [`constants::storage_keys`].
pub mod constants;
/// Contract-level error type.
pub mod error;
/// `InstantiateMsg`, `ExecuteMsg`, `QueryMsg`, `MigrateMsg` definitions.
pub mod msg;
/// Domain types stored in / returned by the contract.
pub mod types;

pub use error::*;
pub use msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
pub use types::*;
