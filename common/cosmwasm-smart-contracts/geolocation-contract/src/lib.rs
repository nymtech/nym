// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod constants;
pub mod error;
pub mod helpers;
pub mod msg;
pub mod types;

/// The canonical location payload. Behind a non-default feature because it carries `f64`
/// coordinates and CosmWasm rejects floating-point instructions at upload; the contract
/// stores payloads opaquely and must never enable this.
#[cfg(feature = "payload")]
pub mod payload;

pub use error::*;
pub use msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
pub use types::*;
