// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials::error::Error as CredentialsError;
use nym_validator_client::{coconut::EcashApiError, nym_api::EpochId};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BandwidthControllerError {
    #[error("Nyxd error: {0}")]
    Nyxd(#[from] nym_validator_client::nyxd::error::NyxdError),

    #[error("coconut api query failure: {0}")]
    CoconutApiError(#[from] EcashApiError),

    #[error("There was a credential storage error - {0}")]
    CredentialStorageError(Box<dyn std::error::Error + Send + Sync>),

    #[error("a credential/global-data fetcher failed - {0}")]
    FetcherError(Box<dyn std::error::Error + Send + Sync>),

    #[error("No expiration date signatures for epoch : {epoch_id}")]
    MissingExpirationDateSignatures { epoch_id: EpochId },

    #[error("No coin index signatures for epoch : {epoch_id}")]
    MissingCoinIndexSignatures { epoch_id: EpochId },

    #[error("No verification key for epoch : {epoch_id}")]
    MissingVerificationKey { epoch_id: EpochId },

    #[error("retrieved upgrade mode token is not a valid String")]
    MalformedUpgradeModeToken,

    #[error("Credential error - {0}")]
    CredentialError(#[from] CredentialsError),

    // Internal error that should not happen, e.g. channel comms failing
    #[error("internal error: {0}")]
    Internal(String),

    #[error("Threshold not set yet")]
    NoThreshold,

    #[error("did not receive a valid response for aggregated data ({typ}) from ANY nym-api")]
    ExhaustedApiQueries { typ: String },
}

impl BandwidthControllerError {
    pub fn credential_storage_error(
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        BandwidthControllerError::CredentialStorageError(Box::new(source))
    }

    pub fn fetcher_error(source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        BandwidthControllerError::FetcherError(source)
    }

    pub fn internal(message: impl ToString) -> Self {
        BandwidthControllerError::Internal(message.to_string())
    }
}
