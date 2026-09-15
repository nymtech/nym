// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_contract_anchor::error::AnchorError;
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
}
