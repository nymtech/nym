// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credential_verification::upgrade_mode::UpgradeModeEnableError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GatewayWireguardError {
    #[error("internal error: {0}")]
    InternalError(String),

    #[error("peers can't be interacted with anymore")]
    PeerInteractionStopped,

    #[error("registration is not in progress for the provided peer key")]
    RegistrationNotInProgress,

    #[error("missing reply_to for old client")]
    MissingReplyToForOldClient,

    #[error("unknown version number")]
    UnknownAuthenticatorVersion,

    #[error("unsupported authenticator version")]
    UnsupportedAuthenticatorVersion,

    #[error("mac does not verify")]
    AuthenticatorMacVerificationFailure,

    #[error("no credential received")]
    MissingAuthenticatorCredential,

    #[error(transparent)]
    UpgradeModeEnable(#[from] UpgradeModeEnableError),

    #[error("credential verification failed: {0}")]
    CredentialVerificationError(#[from] nym_credential_verification::Error),

    #[error(transparent)]
    GatewayStorageError(#[from] nym_gateway_storage::error::GatewayStorageError),

    #[error("failed to serialise authenticator response packet: {source}")]
    AuthenticatorResponseSerialisationFailure { source: Box<bincode::ErrorKind> },
}

impl GatewayWireguardError {
    pub fn internal(message: impl Into<String>) -> Self {
        GatewayWireguardError::InternalError(message.into())
    }

    pub fn authenticator_response_serialisation(
        source: impl Into<Box<bincode::ErrorKind>>,
    ) -> Self {
        GatewayWireguardError::AuthenticatorResponseSerialisationFailure {
            source: source.into(),
        }
    }
}
