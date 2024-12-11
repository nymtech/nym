// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::io;
use std::net::SocketAddr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VerlocError {
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

    #[error("could not establish connection to {identity} on {address}: {err}")]
    UnreachableNode {
        identity: String,
        address: SocketAddr,
        #[source]
        err: io::Error,
    },

    #[error("failed to write echo packet to {identity} on {address}: {err}")]
    UnexpectedConnectionFailureWrite {
        identity: String,
        address: SocketAddr,
        #[source]
        err: io::Error,
    },

    #[error("failed to read reply packet from {identity} on {address}: {err}")]
    UnexpectedConnectionFailureRead {
        identity: String,
        address: SocketAddr,
        #[source]
        err: io::Error,
    },

    #[error("timed out while trying to read reply packet from {identity} on {address}")]
    ConnectionReadTimeout {
        identity: String,
        address: SocketAddr,
    },

    #[error("timed out while trying to write echo packet to {identity} on {address}")]
    ConnectionWriteTimeout {
        identity: String,
        address: SocketAddr,
    },

    #[error("the received reply packet had an unexpected sequence number")]
    UnexpectedReplySequence,

    #[error("shutdown signal received")]
    ShutdownReceived,
}
