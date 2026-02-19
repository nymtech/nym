// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LpTransportError {
    #[error("the encoded packet is too long ({size} bytes)")]
    PacketTooBig { size: usize },

    #[error("the encoded packet is too small ({size} bytes) to encode valid data")]
    PacketTooSmall { size: usize },

    #[error("failed to establish connection with the remote host: {0}")]
    ConnectionFailure(String),

    #[error("failed to configure the established connection: {0}")]
    ConnectionConfigFailure(String),

    #[error("failed to send bytes across the channel: {0}")]
    TransportSendFailure(std::io::Error),

    #[error("failed to receive bytes across the channel: {0}")]
    TransportReceiveFailure(std::io::Error),
}

impl LpTransportError {
    pub fn connection_failure(error: impl Into<String>) -> Self {
        LpTransportError::ConnectionFailure(error.into())
    }

    pub fn connection_config(error: impl Into<String>) -> Self {
        LpTransportError::ConnectionConfigFailure(error.into())
    }

    pub fn send_failure(error: std::io::Error) -> Self {
        LpTransportError::TransportSendFailure(error)
    }

    pub fn receive_failure(error: std::io::Error) -> Self {
        LpTransportError::TransportReceiveFailure(error)
    }
}
