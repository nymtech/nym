// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Common types, messages, errors, and storage-key constants shared between the
//! directory contract and any off-chain client.
//!
//! Keeping these in a separate crate lets clients depend on the contract's public
//! surface (and the canonical signing / digest-leaf encodings) without pulling in
//! `cw-storage-plus` and other on-chain-only dependencies.

/// Contract-wide constants and storage-key namespaces.
pub mod constants;
/// Contract-level error type.
pub mod error;
/// `InstantiateMsg`, `ExecuteMsg`, `QueryMsg`, `MigrateMsg` definitions.
pub mod msg;
/// Domain types, query responses, and canonical encodings.
pub mod types;

pub mod helpers;

pub use error::*;
pub use helpers::node_signing_payload;
pub use msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
pub use types::*;
