// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_bandwidth_controller::{FetcherError, error::FetcherErrorKind};
use nym_credentials::error::Error as CredentialsError;
use nym_validator_client::coconut::EcashApiError;
use thiserror::Error;

#[cfg(all(not(target_arch = "wasm32"), feature = "credentials"))]
use crate::storage::error::StorageError;

#[derive(Debug, Error)]
pub enum NyxdFetcherError {
    #[error("Nyxd error: {0}")]
    Nyxd(#[from] nym_validator_client::nyxd::error::NyxdError),

    #[error("ecash api query failure: {0}")]
    EcashApiError(#[from] EcashApiError),

    #[cfg(all(not(target_arch = "wasm32"), feature = "credentials"))]
    #[error("There was a storage error - {0}")]
    StorageError(#[from] StorageError),

    #[error("Credential error - {0}")]
    CredentialError(#[from] CredentialsError),

    #[cfg(all(not(target_arch = "wasm32"), feature = "credentials"))]
    #[error("Threshold not set yet")]
    NoThreshold,

    #[error("did not receive a valid response for aggregated data ({typ}) from ANY nym-api")]
    ExhaustedApiQueries { typ: String },
}

impl FetcherError for NyxdFetcherError {
    fn kind(&self) -> FetcherErrorKind {
        match self {
            NyxdFetcherError::Nyxd(_)
            | NyxdFetcherError::EcashApiError(_)
            | NyxdFetcherError::ExhaustedApiQueries { .. } => FetcherErrorKind::Api,

            #[cfg(all(not(target_arch = "wasm32"), feature = "credentials"))]
            NyxdFetcherError::StorageError(_) => FetcherErrorKind::Storage,

            #[cfg(all(not(target_arch = "wasm32"), feature = "credentials"))]
            NyxdFetcherError::NoThreshold => FetcherErrorKind::Other,

            NyxdFetcherError::CredentialError(_) => FetcherErrorKind::Other,
        }
    }
}
