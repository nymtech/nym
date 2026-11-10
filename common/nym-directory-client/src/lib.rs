// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Verifiable retrieval of the Nym directory contract.
//!
//! The trust-anchor and ICS23 proof machinery now lives in `nym-contract-anchor`, which is
//! domain-neutral and shared with the other contract clients. Its `anchor`, `error` and
//! `proof` modules are re-exported here so external callers can keep reaching them as
//! `nym_directory_client::anchor::…`, and so this crate's own `crate::anchor::…` /
//! `crate::error::…` / `crate::proof::…` paths resolve. Every read here returns the shared
//! [`VerificationError`](error::VerificationError).
//!
//! Whole-directory reads go through the shared read paths in `nym_contract_anchor::whole_set`
//! (RPC, offline) and `AttestedTrustAnchor::verified_records` (attested); a
//! [`VerifiedDirectory`](verify::VerifiedDirectory) is built from their result with `From`.

pub use nym_contract_anchor::{anchor, error, proof};

pub mod client;
pub mod http;
pub mod key;
pub mod subset;
pub mod verify;

#[cfg(test)]
mod test_support;
