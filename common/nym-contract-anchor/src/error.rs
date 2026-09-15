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

/// Failures of anchoring and proving: establishing which chain state to trust at a height,
/// and proving a raw contract read against it. Domain-neutral - a client for any contract
/// wraps this rather than redefining its variants, so the specific cause survives.
#[derive(Debug, Error)]
pub enum AnchorError {
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

    #[error("light client header verification failed: {0}")]
    LightClientVerificationFailed(String),

    #[error(
        "requested height {requested} precedes the pinned light-client checkpoint at height {checkpoint}"
    )]
    HeightBelowCheckpoint { requested: u64, checkpoint: u64 },

    #[error("non-canonical commit returned for height {0}")]
    NonCanonicalCommit(u64),

    /// The RPC answered a commit query for one height with a (validly signed) commit for a
    /// different one; accepting it would mislabel that header's app hash under the
    /// requested height.
    #[error("commit for height {received} returned when height {requested} was requested")]
    UnexpectedCommitHeight { requested: u64, received: u64 },

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

    /// The root signature over a [`SignedCheckpoint`](crate::anchor::checkpoint::SignedCheckpoint)
    /// did not verify against the configured root key.
    #[error("checkpoint root signature verification failed")]
    InvalidCheckpointSignature,

    /// A checkpoint's carried validator set does not hash to the value committed in its own
    /// signed header, so the datum is internally inconsistent.
    #[error("checkpoint validator set does not match the hash committed in its signed header")]
    CheckpointValidatorMismatch,

    /// The checkpoint's block time is older than the trusting period relative to now, so it
    /// can no longer seed a light client (weak-subjectivity boundary).
    #[error(
        "checkpoint at height {height} is stale: block time is older than the {trusting_period_secs}s trusting period"
    )]
    StaleCheckpoint {
        height: u64,
        trusting_period_secs: u64,
    },

    /// No configured checkpoint source (stored, hardcoded, or HTTPS) yielded a valid,
    /// non-stale checkpoint.
    #[error("no checkpoint source produced a valid, non-stale checkpoint")]
    NoValidCheckpointSource,
}
