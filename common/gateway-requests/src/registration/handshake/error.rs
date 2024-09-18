// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::shared_key::SharedKeyUsageError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HandshakeError {
    #[error("received key material of invalid length: {received}. Expected: {expected}")]
    KeyMaterialOfInvalidSize { received: usize, expected: usize },

    #[error("no nonce has been provided for aes256-gcm-siv key derivation")]
    MissingNonceForCurrentKey,

    #[error(transparent)]
    KeyUsageFailure(#[from] SharedKeyUsageError),

    #[error("received invalid signature")]
    InvalidSignature,
    #[error("encountered network error")]
    NetworkError,
    #[error("encountered network error")]
    ClosedStream,
    #[error("error on the remote: {0}")]
    RemoteError(String),
    #[error("received response was malformed:")]
    MalformedResponse,
    #[error("sent request was malformed")]
    MalformedRequest,
    #[error("sent request was malformed")]
    HandshakeFailure,
    #[error("received shutdown")]
    ReceivedShutdown,

    #[error("timed out waiting for a handshake message")]
    Timeout,
}
