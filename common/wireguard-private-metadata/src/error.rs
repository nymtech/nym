// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum MetadataError {
    #[error("peers can't be interacted with anymore")]
    PeerInteractionStopped,

    #[error("no response received")]
    NoResponse,

    #[error("query was not successful: {reason}")]
    Unsuccessful { reason: String },

    #[error("Models error: {message}")]
    Models { message: String },

    #[error("Credential verification error: {message}")]
    CredentialVerification { message: String },
}

impl From<crate::models::error::Error> for MetadataError {
    fn from(value: crate::models::error::Error) -> Self {
        Self::Models {
            message: value.to_string(),
        }
    }
}

impl From<nym_credential_verification::Error> for MetadataError {
    fn from(value: nym_credential_verification::Error) -> Self {
        Self::CredentialVerification {
            message: value.to_string(),
        }
    }
}
