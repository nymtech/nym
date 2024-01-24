// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::dkg::key_finalization::KeyFinalizationError;
use crate::coconut::error::CoconutError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DkgError {
    #[error("failed to persist local state to disk at path {}: {source}", path.display())]
    StatePersistenceFailure {
        path: PathBuf,
        #[source]
        source: CoconutError,
    },

    #[error("failed to query for the current DKG epoch state: {source}")]
    EpochQueryFailure {
        #[source]
        source: CoconutError,
    },

    #[error("failed to query the CW4 group contract for the membership status: {source}")]
    GroupQueryFailure {
        #[source]
        source: CoconutError,
    },

    #[error("this API is currently not member of the DKG group and thus can't participate in the process")]
    NotInGroup,

    #[error("failed to submit public keys to the DKG contract: {source}")]
    PublicKeySubmissionFailure {
        #[source]
        source: CoconutError,
    },

    #[error("failed to submit DKG dealings to the DKG contract: {source}")]
    DealingExchangeFailure {
        #[source]
        source: CoconutError,
    },

    #[error("failed to submit verification keys to the DKG contract: {source}")]
    VerificationKeySubmissionFailure {
        #[source]
        source: CoconutError,
    },

    #[error("failed to validate verification keys in the DKG contract: {source}")]
    VerificationKeyValidationFailure {
        #[source]
        source: CoconutError,
    },

    #[error("failed to finalize verification keys in the DKG contract: {source}")]
    VerificationKeyFinalizationFailure {
        #[source]
        source: KeyFinalizationError,
    },

    #[error("failed to advance the DKG state: {source}")]
    StateAdvancementFailure {
        #[source]
        source: CoconutError,
    },
}