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

use nym_crypto::asymmetric::ed25519;
use sha2::{Digest, Sha256};

/// Client specific statistics interfaces and events.
pub mod clients;
/// Statistics related errors.
pub mod error;
/// Gateway specific statistics interfaces and events.
pub mod gateways;
/// Statistics reporting abstractions and implementations.
pub mod report;
/// Statistics related types.
pub mod types;

const CLIENT_ID_PREFIX: &str = "client_stats_id";
const VPN_CLIENT_ID_PREFIX: &str = "vpnclient_stats_id";

pub fn generate_client_stats_id(id_key: ed25519::PublicKey) -> String {
    generate_stats_id(CLIENT_ID_PREFIX, id_key.to_base58_string())
}

pub fn generate_vpn_client_stats_id<M: AsRef<[u8]>>(seed: M) -> String {
    generate_stats_id(VPN_CLIENT_ID_PREFIX, seed)
}

fn generate_stats_id<M: AsRef<[u8]>>(prefix: &str, id_seed: M) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(prefix);
    hasher.update(&id_seed);
    let output = hasher.finalize();
    format!("{output:x}")
}

pub fn hash_identifier<M: AsRef<[u8]>>(identifier: M) -> String {
    format!("{:x}", Sha256::digest(identifier))
}
