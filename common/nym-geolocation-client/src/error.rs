// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_contract_anchor::error::{AnchorError, RecordSetError};
use nym_contract_attestation::AttestationSourceError;
use nym_validator_client::nyxd::error::NyxdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GeolocationClientError {
    /// Trust could not be established for the height being read. Wrapped rather than
    /// flattened, so which anchoring step failed survives into the caller's error.
    #[error(transparent)]
    Anchor(#[from] AnchorError),

    /// A plain chain read - the record set or the bonded-node identities at the height -
    /// could not be served. Distinct from [`Self::Anchor`]: establishing the digest is a
    /// trust-anchoring step, fetching the data it is checked against is not.
    #[error("chain query failed: {0}")]
    ChainQuery(#[from] NyxdError),

    /// The accumulator recomputed from the retrieved records does not equal the one trusted
    /// at the verified height, so the set is incomplete, reordered across a write, or
    /// tampered with. Never accompanied by records: there is no partially-verified result.
    #[error(
        "the locally recomputed accumulator does not match the trusted digest at the verified height"
    )]
    DigestMismatch,

    /// The recomputed node-identity hash does not match the attested one. Only the offline
    /// and attested paths can raise this: the RPC-backed path reads identities from the chain
    /// at the same height, so there is nothing to cross-check them against.
    #[error(
        "the locally recomputed node-identity hash does not match the attested value at the verified height"
    )]
    NodeIdentitiesMismatch,

    /// A source answered an attested read for one height with data labelled for another.
    /// Accepting it would stamp the verified result with a height the anchor never established.
    #[error(
        "source served geolocation data for height {received} when height {requested} was requested"
    )]
    SnapshotHeightMismatch { requested: u64, received: u64 },

    /// No attestation source answered an attested read.
    #[error("attestation source transport failure: {0}")]
    AttestationTransport(#[from] AttestationSourceError),

    #[error("no known geolocation contract address was provided")]
    UnavailableGeolocationContract,

    #[error("no known mixnet contract address was provided")]
    UnavailableMixnetContract,
}

impl From<RecordSetError> for GeolocationClientError {
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
