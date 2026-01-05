// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Error types for LP (Lewes Protocol) client operations.

use nym_lp::LpError;
use std::io;
use thiserror::Error;

/// Errors that can occur during LP client operations.
#[derive(Debug, Error)]
pub enum LpClientError {
    /// Failed to establish TCP connection to gateway
    #[error("Failed to connect to gateway at {address}: {source}")]
    TcpConnection {
        address: String,
        #[source]
        source: io::Error,
    },

    /// Failed during LP handshake
    #[error("LP handshake failed: {0}")]
    HandshakeFailed(#[from] LpError),

    /// Failed to send registration request
    #[error("Failed to send registration request: {0}")]
    SendRegistrationRequest(String),

    /// Failed to receive registration response
    #[error("Failed to receive registration response: {0}")]
    ReceiveRegistrationResponse(String),

    /// Registration was rejected by gateway
    #[error("Gateway rejected registration: {reason}")]
    RegistrationRejected { reason: String },

    /// LP transport error
    #[error("LP transport error: {0}")]
    Transport(String),

    /// Invalid LP address format
    #[error("Invalid LP address '{address}': {reason}")]
    InvalidAddress { address: String, reason: String },

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    /// Connection closed unexpectedly
    #[error("Connection closed unexpectedly")]
    ConnectionClosed,

    /// Timeout waiting for response
    #[error("Timeout waiting for {operation}")]
    Timeout { operation: String },

    /// Cryptographic operation failed
    #[error("Cryptographic error: {0}")]
    Crypto(String),
}

impl LpClientError {
    pub fn transport<S>(message: S) -> LpClientError {
        LpClientError::Transport(message.to_string())
    }
}

pub type Result<T> = std::result::Result<T, LpClientError>;
