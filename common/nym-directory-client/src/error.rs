// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_contract_attestation::AttestationSourceError;
use nym_validator_client::error::TendermintRpcError;
use nym_validator_client::nyxd::error::NyxdError;
use thiserror::Error;

// Re-exported so `nym_directory_client::error::{AnchorError, ProofError}` keeps resolving
// for callers that named them here before the anchor machinery moved out.
pub use nym_contract_anchor::error::{AnchorError, ProofError};

#[derive(Debug, Error)]
pub enum DirectoryClientError {
    /// Trust could not be established for the height being read. Wrapped rather than
    /// flattened, so which anchoring step failed survives into the caller's error.
    #[error(transparent)]
    Anchor(#[from] AnchorError),

    /// The digest recomputed from the retrieved entries does not equal the proven
    /// digest, so the set is incomplete or tampered.
    #[error(
        "the locally recomputed digest does not match the proven digest at the verified height"
    )]
    DigestMismatch,

    /// A raw entry value that was proven present on-chain failed to decode with the
    /// contract's value codec (malformed on-chain state).
    #[error("malformed on-chain entry value: {0}")]
    MalformedEntry(String),

    #[error("no known directory contract address was provided")]
    UnavailableDirectoryContract,

    #[error("no known mixnet contract address was provided")]
    UnavailableMixnetContract,

    /// The data-source-agnostic whole-directory verification path
    /// (`verify::verify_directory_offline`) was called without a trusted
    /// node-identities hash to check against - today, only `AttestedTrustAnchor`'s
    /// snapshot carries one.
    #[error("no trusted node-identities hash is available to verify authorship against")]
    NodeIdentitiesHashUnavailable,

    /// A concrete [`AttestationSource`](nym_contract_attestation::AttestationSource) - the
    /// HTTP transport in [`crate::http`] - failed to reach a producer or decode its
    /// response. Surfaced by the client-side subset / whole-directory fetch paths; the
    /// anchor itself treats a failed source as a non-answer and never surfaces this.
    #[error("attestation source transport failure: {0}")]
    AttestationTransport(#[from] AttestationSourceError),

    /// A subset whose canonical bytes matched the quorum-agreed hash still failed to decode
    /// into the expected type via `DirectorySubset::from_canonical_bytes`.
    #[error("malformed subset canonical bytes: {0}")]
    MalformedSubset(String),
}

// The anchoring variants are defined once, in `AnchorError`. These conversions let this
// crate's own `?` sites keep working unchanged without redefining them here.

impl From<NyxdError> for DirectoryClientError {
    fn from(err: NyxdError) -> Self {
        Self::Anchor(AnchorError::ChainQueryFailure(err))
    }
}

impl From<TendermintRpcError> for DirectoryClientError {
    fn from(err: TendermintRpcError) -> Self {
        Self::Anchor(AnchorError::RpcQueryFailure(err))
    }
}

impl From<ProofError> for DirectoryClientError {
    fn from(err: ProofError) -> Self {
        Self::Anchor(AnchorError::Proof(err))
    }
}
