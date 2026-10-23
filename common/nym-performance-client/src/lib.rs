// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Verifiable retrieval of the Nym performance contract's committed scores.
//!
//! The contract commits every node's latest score to an LtHash accumulator stored raw at a
//! provable key. This crate reads the whole committed set at one height and verifies it as a
//! whole, through the chain RPC under any trust anchor or offline against a quorum-attested
//! snapshot, then derives the routing view: every node bonded at that height with its newest
//! committed score and source epoch, or none if it was never measured.
//!
//! The trust anchors, the shared verification steps and the error every read returns
//! ([`VerificationError`](nym_contract_anchor::error::VerificationError)) live in
//! [`nym_contract_anchor`]. A [`VerifiedPerformance`](verify::VerifiedPerformance) is built
//! from any of the shared read paths' result with `From`.

pub mod client;
pub mod http;
pub mod routing;
pub mod verify;

#[cfg(test)]
mod test_support;
