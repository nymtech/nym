// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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

impl HandshakeError {
    pub fn is_network_related(&self) -> bool {
        match self {
            HandshakeError::ClosedStream | HandshakeError::NetworkError => true,
            _ => false,
        }
    }
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
