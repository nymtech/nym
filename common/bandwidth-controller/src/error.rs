// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials::error::Error as CredentialsError;
use nym_validator_client::{coconut::EcashApiError, nym_api::EpochId};
use thiserror::Error;

use crate::traits::CredentialFetcherError;

#[derive(Debug, Error)]
pub enum BandwidthControllerError {
    #[error("Nyxd error: {0}")]
    Nyxd(#[from] nym_validator_client::nyxd::error::NyxdError),

    #[error("coconut api query failure: {0}")]
    CoconutApiError(#[from] EcashApiError),

    #[error("There was a credential storage error - {0}")]
    CredentialStorageError(Box<dyn std::error::Error + Send + Sync>),

    #[error("a credential/global-data fetcher failed - {0}")]
    CredentialFetcherError(CredentialFetcherError),

    #[error("No expiration date signatures for epoch : {epoch_id}")]
    MissingExpirationDateSignatures { epoch_id: EpochId },

    #[error("No coin index signatures for epoch : {epoch_id}")]
    MissingCoinIndexSignatures { epoch_id: EpochId },

    #[error("No verification key for epoch : {epoch_id}")]
    MissingVerificationKey { epoch_id: EpochId },

    #[error("No credential fetcher available")]
    MissingCredentialFetcher,

    #[error("retrieved upgrade mode token is not a valid String")]
    MalformedUpgradeModeToken,

    #[error("Credential error - {0}")]
    CredentialError(#[from] CredentialsError),

    // Internal error that should not happen
    #[error("internal error: {0}")]
    Internal(String),

    #[error("A channel we were using is closed")]
    ChannelClosed,

    #[error("Threshold not set yet")]
    NoThreshold,

    #[error("did not receive a valid response for aggregated data ({typ}) from ANY nym-api")]
    ExhaustedApiQueries { typ: String },

    #[error("failed to parse ticket type: {0}")]
    ParseTicketType(String),

    /// A required type has no usable stock and nothing is being fetched for it - as opposed to a
    /// fetch that actively failed (see [`Self::TicketbookFetchFailed`]).
    #[error("some required ticketbooks are unavailable and none are being fetched")]
    TicketbooksUnavailable,

    /// A required type's fetch actively failed; `reason` is the underlying fetcher error as text.
    #[error("a required ticketbook fetch failed : {reason}")]
    TicketbookFetchFailed { reason: String },
}

impl BandwidthControllerError {
    pub fn credential_storage_error(
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        BandwidthControllerError::CredentialStorageError(Box::new(source))
    }

    pub fn fetcher_error(source: CredentialFetcherError) -> Self {
        BandwidthControllerError::CredentialFetcherError(source)
    }

    pub fn internal(message: impl ToString) -> Self {
        BandwidthControllerError::Internal(message.to_string())
    }
}

/// Coarse category of a fetcher error, so the controller can react to specific cases; anything it
/// doesn't special-case is `Other`.
#[derive(Debug, Clone, Copy)]
pub enum FetcherErrorKind {
    /// the account lacks the funds to make further deposits
    BandwidthDepleted,
    /// a nym-api / ecash query failed
    Api,
    /// an unexpected failure that shouldn't normally happen
    Unexpected,
    /// a local storage failure
    Storage,
    /// anything not worth branching on
    Other,
}
