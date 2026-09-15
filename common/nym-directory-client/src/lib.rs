// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Verifiable retrieval of the Nym directory contract.
//!
//! The trust-anchor and ICS23 proof machinery now lives in `nym-contract-anchor`, which is
//! domain-neutral and shared with the other contract clients. It is re-exported here, so
//! callers that reached it through this crate keep compiling without taking a direct
//! dependency on it - and so this crate's own `crate::anchor::…` / `crate::proof::…` paths
//! keep resolving unchanged.

pub use nym_contract_anchor::anchor::{TrustAnchor, TrustedDigest};
pub use nym_contract_anchor::error::{AnchorError, ProofError};
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
