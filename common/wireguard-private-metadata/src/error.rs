// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::IpAddr;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("peer with IP {ip} doesn't exist")]
    NoPeer { ip: IpAddr },

    #[error("peers can't be interacted with anymore")]
    PeerInteractionStopped,

    #[error("no response received")]
    NoResponse,

    #[error("query was not successful")]
    Unsuccessful,

    #[error("Models error: {message}")]
    Models { message: String },

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

impl From<nym_credential_verification::Error> for Error {
    fn from(value: nym_credential_verification::Error) -> Self {
        Self::CredentialVerification {
            message: value.to_string(),
        }
    }
}
