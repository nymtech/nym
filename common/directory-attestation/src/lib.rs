// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Shared attestation protocol for the Nym directory.
//!
//! This crate is the signer-agnostic home for the wire types and canonical encoders a
//! *producer* signs and a *verifier* (the directory retrieval client) checks:
//!
//! - [`DigestSnapshot`] / [`SignedDigestSnapshot`]: the quorum-signed trust-anchor
//!   bootstrap - a tiny hash-only commitment to a height's `app_hash`, directory digest
//!   `accumulator`, and node-identity binding.
//! - [`DirectorySubset`] / [`SubsetDigest`] / [`SignedSubsetDigest`] / [`AttestedSubset`]:
//!   a generic mechanism for attesting canonical subsets of directory/node data, where a
//!   K-of-N quorum agrees on a small hash and the bulk data is fetched once and verified
//!   by local recompute.
//! - [`AttestationSource`]: the transport contract the retrieval client drives.
//! - [`build_and_sign_snapshot`] / [`sign_subset`]: the signer-agnostic producer core.

pub mod error;
pub mod producer;
pub mod snapshot;
pub mod source;
pub mod subset;

pub use error::AttestationSourceError;
pub use producer::sign_subset;
pub use snapshot::{
    DigestSnapshot, SignedDigestSnapshot, digest_snapshot_signing_payload, node_identities_hash,
};
pub use source::AttestationSource;
pub use subset::{
    AttestedSubset, DirectorySubset, SignedSubsetDigest, SubsetDigest, subset_data_hash,
    subset_digest_signing_payload, subset_hash,
};

/// Append `bytes` prefixed with its u32 little-endian length, so adjacent
/// variable-length fields in a canonical signing payload cannot be confused with one
/// another. Shared by the snapshot and subset encoders.
pub(crate) fn push_len_prefixed(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(bytes);
}
