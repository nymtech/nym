// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Error types for LP (Lewes Protocol) client operations.

use nym_lp::LpError;
use nym_lp::packet::MalformedLpPacketError;
use nym_lp::packet::message::LpMessageType;
use nym_lp::state_machine::LpAction;
use nym_lp::transport::LpTransportError;
use thiserror::Error;

/// Errors that can occur during LP client operations.
#[derive(Debug, Error)]
pub enum LpClientError {
    /// Failed to establish TCP connection to gateway
    #[error("Failed to connect to gateway at {address}: {source}")]
    TcpConnection {
        address: String,
        #[source]
        source: LpTransportError,
    },

    #[error(transparent)]
    LpTransportError(#[from] LpTransportError),

    #[error("the client has not opened a connection to the exit")]
    NotConnected,

    #[error("the KKT/PSQ handshake does not appear to have been completed")]
    IncompleteHandshake,

    #[error(transparent)]
    LpProtocolError(#[from] LpError),

    #[error("no action has been emitted from the LP State Machine")]
    UnexpectedStateMachineHalt,

    #[error("the state machine instructed an unexpected action: {action:?}")]
    UnexpectedStateMachineAction { action: LpAction },

    #[error("received registration data was malformed: {source}")]
    MalformedRegistrationData { source: bincode::Error },

    #[error("received a malformed packet: {0}")]
    MalformedLpPacket(#[from] MalformedLpPacketError),

    #[error("received payload type of an unexpected type: {typ:?}")]
    UnexpectedLpPayload { typ: LpMessageType },

    #[error("timed out while attempting to finish the KKT/PSQ handshake")]
    HandshakeTimeout,

    #[error("timed out while attempting to send to/receive from the connection")]
    ConnectionTimeout,

    /// Failed to send registration request
    #[error("Failed to send registration request: {0}")]
    SendRegistrationRequest(String),

    /// Failed to receive registration response
    #[error("Failed to receive registration response: {0}")]
    ReceiveRegistrationResponse(String),

    /// Registration was rejected by gateway
    #[error("Gateway rejected registration: {reason}")]
    RegistrationRejected { reason: String },

    #[error("could not complete the registration: {message}")]
    RegistrationFailure { message: String },

    #[error("received an unexpected response: {message}")]
    UnexpectedResponse { message: String },

    #[error("currently McEliece keys are not supported for nested registration")]
    UnsupportedNestedMcEliece,

    #[error("{0}")]
    Other(String),
}

impl LpClientError {
    pub fn unexpected_response(message: impl Into<String>) -> LpClientError {
        LpClientError::UnexpectedResponse {
            message: message.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, LpClientError>;
