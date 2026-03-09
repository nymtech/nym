// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MalformedLpPacketError {
    #[error("failed to deserialise received data: {0}")]
    DeserialisationFailure(String),

    #[error("provided insufficient data to fully deserialise the struct")]
    InsufficientData,

    #[error("{0} is not a valid LpFrameKind value")]
    InvalidLpFrameKind(u16),

    #[error("invalid payload size: expected {expected}, got {actual}")]
    InvalidPayloadSize { expected: usize, actual: usize },

    /// Received an LP packet with an incompatible, future, version
    #[error("incompatible LP packet version. got: {got}, highest supported: {highest_supported}")]
    IncompatibleFuturePacketVersion { got: u8, highest_supported: u8 },

    /// Received an LP packet with an incompatible, legacy, version
    #[error("incompatible LP packet version. got: {got}, lowest supported: {lowest_supported}")]
    IncompatibleLegacyPacketVersion { got: u8, lowest_supported: u8 },
}

impl MalformedLpPacketError {
    pub fn invalid_data_kind(frame_kind: u16) -> Self {
        MalformedLpPacketError::InvalidLpFrameKind(frame_kind)
    }
}
