// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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

    #[error("failed to serialize response packet: {source}")]
    FailedToSerializeResponsePacket { source: Box<bincode::ErrorKind> },
}
