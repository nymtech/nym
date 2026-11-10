// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_contract_anchor::error::{AnchorError, ProofError};
use nym_contract_attestation::AttestationSourceError;
use nym_validator_client::error::TendermintRpcError;
use nym_validator_client::nyxd::error::NyxdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DirectoryClientError {
    /// Trust could not be established for the height being read. Wrapped rather than
    /// flattened, so which anchoring step failed survives into the caller's error.
    #[error(transparent)]
    Anchor(#[from] AnchorError),

    /// A plain chain read - the entry set or the bonded-node identities at the height -
    /// could not be served. Distinct from [`Self::Anchor`]: establishing the digest is a
    /// trust-anchoring step, fetching the data it is checked against is not.
    #[error("chain query failed: {0}")]
    ChainQuery(#[from] NyxdError),

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

    /// A source answered a whole-directory fetch for one height with data labelled for
    /// another. The content checks could still pass (they run against the requested
    /// height's trusted values), but accepting it would stamp the verified result with a
    /// height the anchor never established.
    #[error(
        "source served directory data for height {received} when height {requested} was requested"
    )]
    SnapshotHeightMismatch { requested: u64, received: u64 },
}

// `AnchorError` owns the anchoring conversions, so these defer to its `#[from]` rather than
// naming a variant twice. A bare `NyxdError` is handled by the `ChainQuery` variant above: a
// plain data read failing is not an anchoring failure.

impl From<TendermintRpcError> for DirectoryClientError {
    fn from(err: TendermintRpcError) -> Self {
        Self::Anchor(err.into())
    }
}

impl From<ProofError> for DirectoryClientError {
    fn from(err: ProofError) -> Self {
        Self::Anchor(err.into())
    }
}
