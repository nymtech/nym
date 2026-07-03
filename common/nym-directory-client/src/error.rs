// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lthash::DIGEST_LEN;
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
}
