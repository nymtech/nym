// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_validator_client::coconut::EcashApiError;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::error::NyxdError;
use std::io;
use std::net::SocketAddr;
use thiserror::Error;
use time::OffsetDateTime;

#[derive(Debug, Error)]
pub enum VpnApiError {
    #[error("encountered an internal io error: {source}")]
    IoError {
        #[from]
        source: io::Error,
    },

    #[error("could not derive valid client url with the provided webhook parameters")]
    InvalidWebhookUrl,

    #[error("failed to serialise recovery data: {source}")]
    SerdeJsonFailure {
        #[from]
        source: serde_json::Error,
    },

    #[error("the provided expiration date is too late")]
    ExpirationDateTooLate,

    #[error("the provided expiration date is too early")]
    ExpirationDateTooEarly,

    #[error("failed to bind to {address}: {source}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?")]
    SocketBindFailure {
        address: SocketAddr,
        source: io::Error,
    },

    #[error("the api server failed with the following message: {source}")]
    HttpServerFailure { source: io::Error },

    #[error("the ecash contract address is not set")]
    UnavailableEcashContract,

    #[error("the DKG contract address is not set")]
    UnavailableDKGContract,

    #[error("the bandwidth contract doesn't have any admin set")]
    MissingBandwidthContractAdmin,

    #[error(
        "the provided mnemonic does not correspond to the current admin of the bandwidth contract"
    )]
    MismatchedMnemonic,

    #[error("failed to interact with the nyx chain: {source}")]
    NyxdFailure {
        #[from]
        source: NyxdError,
    },

    #[error("validator client error: {0}")]
    ValidatorClientError(#[from] nym_validator_client::ValidatorClientError),

    #[error("failed to perform ecash operation: {source}")]
    EcashApiFailure {
        #[from]
        source: EcashApiError,
    },

    #[error("Compact ecash internal error: {0}")]
    CompactEcashInternalError(#[from] nym_compact_ecash::error::CompactEcashError),

    #[error("there are no rpc endpoints provided in the environment")]
    NoNyxEndpointsAvailable,

    #[error("the threshold value for epoch {epoch_id} is not available")]
    UnavailableThreshold { epoch_id: EpochId },

    #[error(
        "we have only {available} api clients available while the minimum threshold is {threshold}"
    )]
    InsufficientNumberOfSigners { available: usize, threshold: u64 },

    #[error(
    "we have only managed to obtain {available} partial credentials while the minimum threshold is {threshold}"
    )]
    InsufficientNumberOfCredentials { available: usize, threshold: u64 },

    #[error("failed to interact with the credentials: {source}")]
    CredentialsFailure {
        #[from]
        source: nym_credentials::Error,
    },

    #[error("the DKG has not yet been initialised in the system")]
    UninitialisedDkg,

    #[error("credentials can't yet be issued in the system. approximate expected availability: {availability}")]
    CredentialsNotYetIssuable { availability: OffsetDateTime },

    #[error("reached seemingly impossible ecash failure")]
    UnknownEcashFailure,

    #[error("experienced internal database error: {0}")]
    InternalDatabaseError(#[from] sqlx::Error),

    #[error("experienced internal storage error: {reason}")]
    DatabaseInconsistency { reason: String },

    #[error("failed to perform startup SQL migration: {0}")]
    StartupMigrationFailure(#[from] sqlx::migrate::MigrateError),

    #[error("timed out while attempting to obtain partial wallet from {client_repr}")]
    EcashApiRequestTimeout { client_repr: String },
}

impl VpnApiError {
    pub fn database_inconsistency<S: Into<String>>(reason: S) -> VpnApiError {
        VpnApiError::DatabaseInconsistency {
            reason: reason.into(),
        }
    }
}
