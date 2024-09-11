// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
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

    #[error("the provided base64-encoded client x25519 public key ('{pub_key}') was malformed: {source}")]
    MalformedPeerPublicKeyEncoding {
        pub_key: String,
        #[source]
        source: base64::DecodeError,
    },

    #[error("the provided base64-encoded client x25519 public key ('{pub_key}') has invalid length: {decoded_length}. expected 32 bytes")]
    InvalidPeerPublicKeyLength {
        pub_key: String,
        decoded_length: usize,
    },

    #[cfg(feature = "verify")]
    #[error("failed to verify mac provided by '{client}': {source}")]
    FailedClientMacVerification {
        client: String,
        #[source]
        source: hmac::digest::MacError,
    },
}
