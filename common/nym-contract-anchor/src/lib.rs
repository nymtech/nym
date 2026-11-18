// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Trust anchors and proof verification for Nym contract state.
//!
//! Domain-neutral by construction: everything here is parameterised by a contract address
//! and a digest storage key rather than reaching for a per-domain constant, so one copy of
//! the ICS23 machinery and the four anchors serves every contract that maintains an LtHash
//! accumulator at a raw storage key.
//!
//! - [`proof`]: two-layer ICS23 wasm-store membership, non-membership and presence checks
//!   against a trusted block `app_hash`.
//! - [`anchor`]: the [`TrustAnchor`](anchor::TrustAnchor) trait and its four
//!   implementations - proven, light-client, attested, and the checkpoint bootstrap that
//!   seeds the light client.
//! - [`whole_set`]: verifying a contract's whole record set against trusted values, generic
//!   over the record's leaf, and the bond set a client reads alongside it.
//! - [`error`]: [`VerificationError`](error::VerificationError), every way a verified read of
//!   contract data can fail, which every contract client returns, including
//!   [`AnchorError`](error::AnchorError) for anchoring and proving.

pub mod anchor;
pub mod error;
pub mod proof;
pub mod whole_set;

#[cfg(test)]
mod test_support;
