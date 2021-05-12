// Copyright 2021 Nym Technologies SA
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

use std::fmt::{self, Display, Formatter};
use std::io;

#[derive(Debug)]
pub enum RttError {
    UnexpectedEchoPacketSize,
    UnexpectedReplyPacketSize,

    MalformedSenderIdentity,

    MalformedEchoSignature,
    MalformedReplySignature,

    InvalidEchoSignature,
    InvalidReplySignature,

    UnreachableNode(String, io::Error),
    UnexpectedConnectionFailureWrite(String, io::Error),
    UnexpectedConnectionFailureRead(String, io::Error),
    ConnectionReadTimeout(String),

    UnexpectedReplySequence,
}

impl Display for RttError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RttError::UnexpectedEchoPacketSize => {
                write!(f, "The received echo packet had unexpected size")
            }
            RttError::UnexpectedReplyPacketSize => {
                write!(f, "The received reply packet had unexpected size")
            }
            RttError::MalformedSenderIdentity => {
                write!(f, "The received echo packet had malformed sender")
            }
            RttError::MalformedEchoSignature => {
                write!(f, "The received echo packet had malformed signature")
            }
            RttError::MalformedReplySignature => {
                write!(f, "The received reply packet had malformed signature")
            }
            RttError::InvalidEchoSignature => {
                write!(f, "The received echo packet had invalid signature")
            }
            RttError::InvalidReplySignature => {
                write!(f, "The received reply packet had invalid signature")
            }
            RttError::UnreachableNode(id, err) => {
                write!(f, "Could not establish connection to {} - {}", id, err)
            }
            RttError::UnexpectedConnectionFailureWrite(id, err) => {
                write!(f, "Failed to write echo packet to {} - {}", id, err)
            }
            RttError::UnexpectedConnectionFailureRead(id, err) => {
                write!(f, "Failed to read reply packet from {} - {}", id, err)
            }
            RttError::ConnectionReadTimeout(id) => {
                write!(f, "Timed out while trying to read reply packet from {}", id)
            }
            RttError::UnexpectedReplySequence => write!(
                f,
                "The received reply packet had an unexpected sequence number"
            ),
        }
    }
}

impl std::error::Error for RttError {}
