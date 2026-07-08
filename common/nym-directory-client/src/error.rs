// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lthash::DIGEST_LEN;
use nym_validator_client::error::TendermintRpcError;
use nym_validator_client::nyxd::error::NyxdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProofError {
    #[error("expected exactly 2 proof ops (ics23:iavl, ics23:simple), got {0}")]
    UnexpectedOpCount(usize),

    #[error("failed to decode the ICS23 commitment proof for op {op}: {source}")]
    Decode {
        op: usize,
        source: prost::DecodeError,
    },

    #[error("proof op {0} is not an existence proof")]
    NotExistenceProof(usize),

    #[error("failed to compute the existence root: {0}")]
    RootCalculation(String),

    #[error(
        "IAVL-layer membership verification failed (key/value not committed in the wasm store)"
    )]
    IavlVerificationFailed,

    #[error(
        "store-layer membership verification failed (wasm store not committed to the app_hash)"
    )]
    StoreVerificationFailed,
}

#[derive(Debug, Error)]
pub enum DirectoryClientError {
    #[error("chain query failed: {0}")]
    ChainQueryFailure(#[from] NyxdError),

    #[error("rpc query failed: {0}")]
    RpcQueryFailure(#[from] TendermintRpcError),

    #[error(transparent)]
    Proof(#[from] ProofError),

    #[error(
        "digest item has unexpected length {0} (expected a {DIGEST_LEN}-byte LtHash accumulator)"
    )]
    BadDigestLength(usize),

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

    #[error("light client header verification failed: {0}")]
    LightClientVerificationFailed(String),

    #[error(
        "requested height {requested} precedes the pinned light-client checkpoint at height {checkpoint}"
    )]
    HeightBelowCheckpoint { requested: u64, checkpoint: u64 },

    #[error("non-canonical commit returned for height {0}")]
    NonCanonicalCommit(u64),

    /// Fewer than `needed` distinct trusted signers agreed on identical attested
    /// values (or none did). `agreed` is the largest distinct-signer count seen across
    /// any single value grouping, so callers can see how close the quorum came.
    #[error("quorum not reached: needed {needed} distinct trusted signers, got {agreed}")]
    QuorumNotReached { needed: usize, agreed: usize },

    /// No quorum-agreed attestation exists for the requested height. This can be
    /// transient (a source has not yet, or no longer, holds that height) or permanent
    /// (the height was never a real snapshot point) - the anchor cannot always tell
    /// which, since a requested height only ever comes from a real observed snapshot
    /// (self-seeded during `refresh`, or externally supplied by a caller with
    /// independent reason to trust it exists), never guessed.
    #[error("no quorum-agreed snapshot exists for height {0}")]
    NoQuorumSnapshotForHeight(u64),

    /// `AttestedTrustAnchor::new` was called with a degenerate quorum threshold.
    #[error("invalid quorum configuration: quorum {quorum} with {signers} trusted signers")]
    InvalidQuorumConfig { quorum: usize, signers: usize },

    /// The data-source-agnostic whole-directory verification path
    /// (`verify::verify_directory_offline`) was called without a trusted
    /// node-identities hash to check against - today, only `AttestedTrustAnchor`'s
    /// snapshot carries one.
    #[error("no trusted node-identities hash is available to verify authorship against")]
    NodeIdentitiesHashUnavailable,
}
