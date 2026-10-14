// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_contract_anchor::error::{AnchorError, RecordSetError};
use nym_contract_attestation::AttestationSourceError;
use nym_validator_client::nyxd::error::NyxdError;
use thiserror::Error;

/// No variant carries entries: a failed verification returns no data.
#[derive(Debug, Error)]
pub enum PerformanceClientError {
    /// Trust could not be established for the height being read. Wrapped rather than
    /// flattened, so which anchoring step failed survives into the caller's error.
    #[error(transparent)]
    Anchor(#[from] AnchorError),

    /// A plain chain read - the committed entries or the bonded-node identities at the
    /// height - could not be served.
    #[error("chain query failed: {0}")]
    ChainQuery(#[from] NyxdError),

    /// The accumulator recomputed from the served entries does not equal the one trusted at
    /// the verified height, so the set is incomplete or tampered with.
    #[error(
        "the locally recomputed accumulator does not match the trusted digest at the verified height"
    )]
    DigestMismatch,

    /// The served node-identity map does not hash to the attested value. Only the offline
    /// and attested paths can raise this.
    #[error(
        "the locally recomputed node-identity hash does not match the attested value at the verified height"
    )]
    NodeIdentitiesMismatch,

    /// A source answered an attested read for one height with data labelled for another.
    #[error(
        "source served performance data for height {received} when height {requested} was requested"
    )]
    SnapshotHeightMismatch { requested: u64, received: u64 },

    /// No attestation source answered an attested read.
    #[error("attestation source transport failure: {0}")]
    AttestationTransport(#[from] AttestationSourceError),

    #[error("no known performance contract address was provided")]
    UnavailablePerformanceContract,

    #[error("no known mixnet contract address was provided")]
    UnavailableMixnetContract,
}

impl From<RecordSetError> for PerformanceClientError {
    fn from(err: RecordSetError) -> Self {
        match err {
            RecordSetError::DigestMismatch => Self::DigestMismatch,
            RecordSetError::NodeIdentitiesMismatch => Self::NodeIdentitiesMismatch,
            RecordSetError::SnapshotHeightMismatch {
                requested,
                received,
            } => Self::SnapshotHeightMismatch {
                requested,
                received,
            },
            RecordSetError::Anchor(err) => Self::Anchor(err),
            RecordSetError::Transport(err) => Self::AttestationTransport(err),
        }
    }
}
