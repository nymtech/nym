// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MalformedLpPacketError {
    #[error("failed to deserialise received data: {0}")]
    DeserialisationFailure(String),

    #[error("provided insufficient data to fully deserialise the struct")]
    InsufficientData,

    #[error("invalid payload size: expected {expected}, got {actual}")]
    InvalidPayloadSize { expected: usize, actual: usize },

    /// Received an LP packet whose version isn't the one negotiated for its session
    #[error("unexpected LP packet version. got: {got}, expected: {expected}")]
    UnexpectedPacketVersion { got: u8, expected: u8 },

    /// Negotiated an LP version whose header layout this build doesn't implement
    #[error("unsupported LP packet version: {got}")]
    UnsupportedPacketVersion { got: u8 },
}
