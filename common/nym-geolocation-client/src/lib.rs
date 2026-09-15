// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Verifiable retrieval of the Nym geolocation contract.
//!
//! The contract commits every entry to an LtHash accumulator at a raw, ICS23-provable
//! storage key, so a client can establish for itself that what it read is the complete,
//! unmodified set at a height rather than trusting whoever served it.
//!
//! Verification here is **whole-set**. The entries key is `(subject_class, subject_id,
//! source)`, so answering "where is node 42" is a prefix scan, and ICS23 proves membership
//! and non-membership of a key rather than completeness over a range. Only recomputing the
//! accumulator over every record establishes completeness for a subject, which is what the
//! consumers actually need; proven single-entry reads are deliberately not offered.
//!
//! The trust anchors and the proof machinery are shared with the other contract clients and
//! live in [`nym_contract_anchor`].

pub mod attestation;
pub mod client;
pub mod error;
pub mod http;
pub mod key;
pub mod verified;
pub mod verify;
pub mod whitelist;

#[cfg(test)]
mod test_records;

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod proof_tests;
