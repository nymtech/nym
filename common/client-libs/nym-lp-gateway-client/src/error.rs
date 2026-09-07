// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Error types for LP (Lewes Protocol) client operations.

use nym_api_requests::models::described::type_translation::MalformedLPData;
use nym_lp::LpError;
use nym_lp::session::LpAction;
use nym_lp::transport::LpTransportError;
use nym_lp_data::packet::MalformedLpPacketError;
use nym_lp_data::packet::frame::LpFrameKind;
use std::net::SocketAddr;
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

    #[error("there is no open control connection to the gateway at {gateway}")]
    NotConnected { gateway: SocketAddr },

    #[error("this client has no LP data socket; it was built for control traffic only")]
    NoDataSocket,

    #[error("the node does not have LP enabled")]
    LpNotEnabled,

    #[error("the node publishes no LP details to reach it by")]
    NoLpDetailsPublished,

    #[error("the node published malformed LP details: {source}")]
    MalformedLpNodeDetails {
        #[source]
        source: MalformedLPData,
    },

    #[error("a node built from {build_version} speaks no version of LP")]
    NoLpForBuildVersion { build_version: String },

    #[error(
        "the gateway speaks LP protocol version {advertised}, which this build no longer supports"
    )]
    UnsupportedProtocolVersion { advertised: u8 },

    #[error("the KKT/PSQ handshake does not appear to have been completed")]
    IncompleteHandshake,

    #[error(transparent)]
    LpProtocolError(#[from] LpError),

    #[error("the state machine instructed an unexpected action: {action:?}")]
    UnexpectedStateMachineAction { action: LpAction },

    #[error("received registration data was malformed: {source}")]
    MalformedRegistrationData { source: bincode::Error },

    #[error("received a malformed packet: {0}")]
    MalformedLpPacket(#[from] MalformedLpPacketError),

    #[error("received payload type of an unexpected type: {typ:?}")]
    UnexpectedLpPayload { typ: LpFrameKind },

    #[error("timed out while attempting to finish the KKT/PSQ handshake")]
    HandshakeTimeout,

    #[error("timed out while attempting to send to/receive from the connection")]
    ConnectionTimeout,

    #[error("No {ticketbook_type} tickets available")]
    NoTicketsAvailable { ticketbook_type: String },

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
