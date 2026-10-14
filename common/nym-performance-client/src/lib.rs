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
//! The trust anchors and the shared verification steps live in [`nym_contract_anchor`].

pub mod error;
