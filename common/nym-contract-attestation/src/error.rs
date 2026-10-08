// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

/// A failure while talking to an [`AttestationSource`](crate::AttestationSource).
///
/// Kept deliberately small: a consumer (the directory retrieval client) wraps this in
/// its own richer error, and the anchor's quorum logic treats a failed source as a
/// non-answer rather than surfacing the specific failure.
#[derive(Debug, thiserror::Error)]
pub enum AttestationSourceError {
    /// The source could not be reached or returned a response that could not be decoded.
    #[error("failed to communicate with the attestation source: {0}")]
    Transport(String),

    /// The source currently has no snapshot at all (e.g. it has not produced one yet).
    #[error("the attestation source has no snapshot available")]
    NoSnapshotAvailable,

    /// The source has no snapshot at the requested height (outside its retained window).
    #[error("the attestation source has no snapshot at height {height}")]
    NoSnapshotAtHeight { height: u64 },
}
