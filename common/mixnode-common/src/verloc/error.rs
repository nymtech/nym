// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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
    ConnectionWriteTimeout(String),

    UnexpectedReplySequence,

    ShutdownReceived,
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
            RttError::ConnectionWriteTimeout(id) => {
                write!(f, "Timed out while trying to write echo packet to {}", id)
            }
            RttError::UnexpectedReplySequence => write!(
                f,
                "The received reply packet had an unexpected sequence number"
            ),
            RttError::ShutdownReceived => {
                write!(f, "Shutdown signal received")
            }
        }
    }
}

impl std::error::Error for RttError {}
