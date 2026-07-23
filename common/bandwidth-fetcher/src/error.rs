// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_bandwidth_controller::{FetcherError, TicketType, error::FetcherErrorKind};
use nym_credentials::error::Error as CredentialsError;
use nym_validator_client::coconut::EcashApiError;
use thiserror::Error;

#[cfg(not(target_arch = "wasm32"))]
use crate::storage::error::StorageError;

#[derive(Debug, Error)]
pub enum NyxdFetcherError {
    #[error("Nyxd error: {0}")]
    Nyxd(#[from] nym_validator_client::nyxd::error::NyxdError),

    #[error("ecash api query failure: {0}")]
    EcashApiError(#[from] EcashApiError),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("There was a storage error - {0}")]
    StorageError(#[from] StorageError),

    #[error("Credential error - {0}")]
    CredentialError(#[from] CredentialsError),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("Threshold not set yet")]
    NoThreshold,

    #[error("did not receive a valid response for aggregated data ({typ}) from ANY nym-api")]
    ExhaustedApiQueries { typ: String },

    /// A fresh deposit was requested for a ticketbook type outside the fetcher's allowed set.
    /// Recovery of already-paid deposits is never blocked by this.
    #[error("ticketbook type {requested} is not permitted by this fetcher (allowed: {allowed:?})")]
    TicketbookTypeNotAllowed {
        requested: TicketType,
        allowed: Vec<TicketType>,
    },
}

impl FetcherError for NyxdFetcherError {
    fn kind(&self) -> FetcherErrorKind {
        match self {
            NyxdFetcherError::Nyxd(_)
            | NyxdFetcherError::EcashApiError(_)
            | NyxdFetcherError::ExhaustedApiQueries { .. } => FetcherErrorKind::Api,

            #[cfg(not(target_arch = "wasm32"))]
            NyxdFetcherError::StorageError(_) => FetcherErrorKind::Storage,

            #[cfg(not(target_arch = "wasm32"))]
            NyxdFetcherError::NoThreshold => FetcherErrorKind::Other,

            NyxdFetcherError::CredentialError(_) => FetcherErrorKind::Other,

            // Deliberately mapped to `Other` (not a new `FetcherErrorKind` variant): downstream
            // consumers may match `FetcherErrorKind` exhaustively, and it is not `#[non_exhaustive]`.
            NyxdFetcherError::TicketbookTypeNotAllowed { .. } => FetcherErrorKind::Other,
        }
    }
}
