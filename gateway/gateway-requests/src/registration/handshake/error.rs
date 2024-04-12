// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::identity;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum HandshakeError {
    #[error(
        "received key material of invalid length - {0}. Expected: {}",
        identity::SIGNATURE_LENGTH
    )]
    KeyMaterialOfInvalidSize(usize),
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

    #[error("timed out waiting for a handshake message")]
    Timeout,
}
