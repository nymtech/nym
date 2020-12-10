// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crypto::asymmetric::identity;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum HandshakeError {
    KeyMaterialOfInvalidSize(usize),
    InvalidSignature,
    NetworkError,
    ClosedStream,
    RemoteError(String),
    MalformedResponse,
    MalformedRequest,
    HandshakeFailure,
}

impl Display for HandshakeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            HandshakeError::KeyMaterialOfInvalidSize(received) => write!(
                f,
                "received key material of invalid length - {}. Expected: {}",
                received,
                identity::SIGNATURE_LENGTH
            ),
            HandshakeError::InvalidSignature => write!(f, "received invalid signature"),
            HandshakeError::NetworkError => write!(f, "encountered network error"),
            HandshakeError::ClosedStream => {
                write!(f, "the stream was closed before completing handshake")
            }
            HandshakeError::RemoteError(err) => write!(f, "error on the remote: {}", err),
            HandshakeError::MalformedResponse => write!(f, "received response was malformed:"),
            HandshakeError::MalformedRequest => write!(f, "sent request was malformed"),
            HandshakeError::HandshakeFailure => write!(f, "unknown handshake failure"),
        }
    }
}

impl std::error::Error for HandshakeError {}
