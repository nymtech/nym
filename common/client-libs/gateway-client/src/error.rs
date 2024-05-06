// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(target_arch = "wasm32")]
use gloo_utils::errors::JsError;
use nym_gateway_requests::registration::handshake::error::HandshakeError;
use std::io;
use thiserror::Error;
use tungstenite::Error as WsError;

#[derive(Debug, Error)]
pub enum GatewayClientError {
    #[error("Connection to the gateway is not established")]
    ConnectionNotEstablished,

    #[error("Gateway returned an error response: {0}")]
    GatewayError(String),

    #[error("There was a network error: {0}")]
    NetworkError(#[from] WsError),

    #[cfg(target_arch = "wasm32")]
    #[error("There was a network error: {0}")]
    NetworkErrorWasm(#[from] JsError),

    #[error("Invalid URL: {0}")]
    InvalidURL(String),

    #[error("No shared key was provided or obtained")]
    NoSharedKeyAvailable,

    #[error("No bandwidth controller provided")]
    NoBandwidthControllerAvailable,

    #[error("Bandwidth controller error: {0}")]
    BandwidthControllerError(#[from] nym_bandwidth_controller::error::BandwidthControllerError),

    #[error("Connection was abruptly closed")]
    ConnectionAbruptlyClosed,

    #[error("Connection was abruptly closed as gateway was stopped")]
    ConnectionClosedGatewayShutdown,

    #[error("Received response was malformed")]
    MalformedResponse,

    #[error("Credential could not be serialized")]
    SerializeCredential,

    #[error("can not spend bandwidth credential with the gateway as it's using outdated protocol (version: {negotiated_protocol:?})")]
    OutdatedGatewayCredentialVersion { negotiated_protocol: Option<u8> },

    #[error("Client is not authenticated")]
    NotAuthenticated,

    #[error("Client does not have enough bandwidth: estimated {0}, remaining: {1}")]
    NotEnoughBandwidth(i64, i64),

    #[error("There are no more bandwidth credentials acquired. Please buy some more if you want to use the mixnet")]
    NoMoreBandwidthCredentials,

    #[error("Received an unexpected response")]
    UnexpectedResponse,

    #[error("Connection is in an invalid state - please send a bug report")]
    ConnectionInInvalidState,

    #[error("Failed to finish registration handshake: {0}")]
    RegistrationFailure(HandshakeError),

    #[error("Authentication failure")]
    AuthenticationFailure,

    #[error("Timed out")]
    Timeout,

    #[error("timeout sending ws message to the gateway")]
    TimeoutOnSendingWs,

    #[error("Failed to send mixnet message")]
    MixnetMsgSenderFailedToSend,

    #[error("Attempted to negotiate connection with gateway using incompatible protocol version. Ours is {current} and the gateway reports {gateway:?}")]
    IncompatibleProtocol { gateway: Option<u8>, current: u8 },

    #[error(
        "The packet router hasn't been set - are you sure you started up the client correctly?"
    )]
    PacketRouterUnavailable,

    #[error(
        "this operation couldn't be completed as the program is in the process of shutting down"
    )]
    ShutdownInProgress,
}

impl GatewayClientError {
    pub fn is_closed_connection(&self) -> bool {
        log::info!("GatewayClientError::is_closed_connection");
        match self {
            GatewayClientError::NetworkError(ws_err) => match ws_err {
                WsError::AlreadyClosed | WsError::ConnectionClosed => true,
                WsError::Io(io_err) => matches!(
                    io_err.kind(),
                    io::ErrorKind::ConnectionReset
                        | io::ErrorKind::ConnectionAborted
                        | io::ErrorKind::BrokenPipe
                ),
                _ => false,
            },
            _ => false,
        }
    }
}
