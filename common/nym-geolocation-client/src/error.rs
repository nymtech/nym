// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_contract_anchor::error::AnchorError;
use nym_validator_client::nyxd::error::NyxdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GeolocationClientError {
    /// Trust could not be established for the height being read. Wrapped rather than
    /// flattened, so which anchoring step failed survives into the caller's error.
    #[error(transparent)]
    Anchor(#[from] AnchorError),

    /// The accumulator recomputed from the retrieved records does not equal the one trusted
    /// at the verified height, so the set is incomplete, reordered across a write, or
    /// tampered with. Never accompanied by records: there is no partially-verified result.
    #[error(
        "the locally recomputed accumulator does not match the trusted digest at the verified height"
    )]
    DigestMismatch,

    /// The recomputed node-identity hash does not match the attested one. Only the offline
    /// path can raise this: the RPC-backed path reads identities from the chain at the same
    /// height, so there is nothing to cross-check them against.
    #[error(
        "the locally recomputed node-identity hash does not match the attested value at the verified height"
    )]
    NodeIdentitiesMismatch,

    #[error("no known geolocation contract address was provided")]
    UnavailableGeolocationContract,

    #[error("no known mixnet contract address was provided")]
    UnavailableMixnetContract,
}

// `AnchorError` owns the chain-query variants; this lets the client's own `?` sites reach
// them without redefining any.
impl From<NyxdError> for GeolocationClientError {
    fn from(err: NyxdError) -> Self {
        Self::Anchor(AnchorError::ChainQueryFailure(err))
    }
}
