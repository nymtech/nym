// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Display;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("the provided base64-encoded client MAC ('{mac}') was malformed: {source}")]
    MalformedClientMac {
        mac: String,
        #[source]
        source: base64::DecodeError,
    },

    #[cfg(feature = "verify")]
    #[error("failed to verify mac provided by '{client}': {source}")]
    FailedClientMacVerification {
        client: String,
        #[source]
        source: hmac::digest::MacError,
    },

    #[error("conversion: {0}")]
    Conversion(String),

    // TODO add version number for debugging
    #[error("unknown version number")]
    UnknownVersion,

    // TODO add version number for debugging
    #[error("unsupported request version")]
    UnsupportedVersion,

    #[error("gateway doesn't support this type of message")]
    UnsupportedMessage,

    #[error(transparent)]
    Bincode(#[from] bincode::Error),
}

impl Error {
    pub fn conversion(msg: impl Into<String>) -> Self {
        Error::Conversion(msg.into())
    }

    pub fn conversion_display(msg: impl Display) -> Self {
        Error::Conversion(msg.to_string())
    }
}
