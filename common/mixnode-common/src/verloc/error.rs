// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RttError {
    #[error("the received echo packet had unexpected size")]
    UnexpectedEchoPacketSize,

    #[error("the received reply packet had unexpected size")]
    UnexpectedReplyPacketSize,

    #[error("the received echo packet had malformed sender")]
    MalformedSenderIdentity,

    #[error("the received echo packet had malformed signature")]
    MalformedEchoSignature,

    #[error("the received reply packet had malformed signature")]
    MalformedReplySignature,

    #[error("the received echo packet had invalid signature")]
    InvalidEchoSignature,

    #[error("the received reply packet had invalid signature")]
    InvalidReplySignature,

    #[error("could not establish connection to {0}: {1}")]
    UnreachableNode(String, #[source] io::Error),

    #[error("failed to write echo packet to {0}: {1}")]
    UnexpectedConnectionFailureWrite(String, #[source] io::Error),

    #[error("failed to read reply packet from {0}: {1}")]
    UnexpectedConnectionFailureRead(String, #[source] io::Error),

    #[error("timed out while trying to read reply packet from {0}")]
    ConnectionReadTimeout(String),

    #[error("timed out while trying to write echo packet to {0}")]
    ConnectionWriteTimeout(String),

    #[error("the received reply packet had an unexpected sequence number")]
    UnexpectedReplySequence,

    #[error("shutdown signal received")]
    ShutdownReceived,
}
