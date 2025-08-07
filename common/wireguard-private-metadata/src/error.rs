// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[cfg(feature = "server")]
    #[error("peers can't be interacted with anymore")]
    PeerInteractionStopped,

    #[cfg(feature = "server")]
    #[error("no response received")]
    NoResponse,

    #[cfg(feature = "server")]
    #[error("query was not successful: {reason}")]
    Unsuccessful { reason: String },

    #[error("Models error: {message}")]
    Models { message: String },

    #[cfg(feature = "server")]
    #[error("Credential verification error: {message}")]
    CredentialVerification { message: String },
}

impl From<crate::models::error::Error> for Error {
    fn from(value: crate::models::error::Error) -> Self {
        Self::Models {
            message: value.to_string(),
        }
    }
}

#[cfg(feature = "server")]
impl From<nym_credential_verification::Error> for Error {
    fn from(value: nym_credential_verification::Error) -> Self {
        Self::CredentialVerification {
            message: value.to_string(),
        }
    }
}
