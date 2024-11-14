// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Nym Statistics
//!
//! This crate contains basic statistics utilities and abstractions to be re-used and
//! applied throughout both the client and gateway implementations.

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

use sha2::Digest;

/// Client specific statistics interfaces and events.
pub mod clients;
/// Statistics related errors.
pub mod error;
/// Gateway specific statistics interfaces and events.
pub mod gateways;
/// Statistics reporting abstractions and implementations.
pub mod report;

const CLIENT_ID_PREFIX: &str = "client_stats_id";

pub fn generate_client_stats_id(id_key: &str) -> String {
    generate_stats_id(CLIENT_ID_PREFIX, id_key)
}

fn generate_stats_id(prefix: &str, id_key: &str) -> String {
    let mut hash_input = prefix.to_owned();
    hash_input.push_str(id_key);
    format!("{:x}", sha2::Sha256::digest(hash_input))
}
