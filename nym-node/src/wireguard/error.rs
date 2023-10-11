// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use hmac::digest::MacError;
use std::net::AddrParseError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WireguardError {
    #[error("the provided base64-encoded client MAC ('{mac}') was malformed: {source}")]
    MalformedClientMac {
        mac: String,
        #[source]
        source: base64::DecodeError,
    },

    #[error("the provided base64-encoded client x25519 public key ('{pub_key}') was malformed: {source}")]
    MalformedClientPublicKeyEncoding {
        pub_key: String,
        #[source]
        source: base64::DecodeError,
    },

    #[error("the provided base64-encoded client x25519 public key ('{pub_key}') has invalid length: {decoded_length}. expected 32 bytes")]
    InvalidClientPublicKeyLength {
        pub_key: String,
        decoded_length: usize,
    },

    #[error("failed to verify mac provided by '{client}': {source}")]
    FailedClientMacVerification {
        client: String,
        #[source]
        source: MacError,
    },

    #[error("the provided client socket address ('{raw}') was malformed: {source}")]
    MalformedClientSocketAddress {
        raw: String,
        #[source]
        source: AddrParseError,
    },
}
