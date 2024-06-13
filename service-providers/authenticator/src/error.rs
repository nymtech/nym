// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::error::ClientCoreError;
use nym_id::NymIdError;

#[derive(thiserror::Error, Debug)]
pub enum AuthenticatorError {
    #[error("client-core error: {0}")]
    ClientCoreError(#[from] ClientCoreError),

    // TODO: add more details here
    #[error("failed to validate the loaded config")]
    ConfigValidationFailure,

    #[error("the entity wrapping the network requester has disconnected")]
    DisconnectedParent,

    #[error("failed local version check, client and config mismatch")]
    FailedLocalVersionCheck,

    #[error("failed to connect to mixnet: {source}")]
    FailedToConnectToMixnet { source: nym_sdk::Error },

    #[error("failed to load configuration file: {0}")]
    FailedToLoadConfig(String),

    #[error("failed to setup mixnet client: {source}")]
    FailedToSetupMixnetClient { source: nym_sdk::Error },

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    NymIdError(#[from] NymIdError),
}

pub type Result<T> = std::result::Result<T, AuthenticatorError>;
