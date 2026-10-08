// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Verifiable retrieval of the Nym directory contract.
//!
//! The trust-anchor and ICS23 proof machinery now lives in `nym-contract-anchor`, which is
//! domain-neutral and shared with the other contract clients. Its `anchor` and `proof`
//! modules are re-exported here so the pre-existing external callers that reached them as
//! `nym_directory_client::anchor::…` keep compiling, and so this crate's own
//! `crate::anchor::…` / `crate::proof::…` paths keep resolving unchanged.

pub use nym_contract_anchor::{anchor, proof};

pub mod attested_directory;
pub mod client;
pub mod error;
pub mod http;
pub mod key;
pub mod subset;
pub mod verify;

#[cfg(test)]
mod test_support;
