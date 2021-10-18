// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use gateway_requests::registration::handshake::error::HandshakeError;
use std::io;
use thiserror::Error;
use tungstenite::Error as WsError;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

#[derive(Debug, Error)]
pub enum GatewayClientError {
    #[error("Connection to the gateway is not established")]
    ConnectionNotEstablished,

    #[error("Gateway returned an error response - {0}")]
    GatewayError(String),

    #[error("There was a network error - {0}")]
    NetworkError(#[from] WsError),

    // TODO: see if `JsValue` is a reasonable type for this
    #[cfg(target_arch = "wasm32")]
    #[error("There was a network error - {0}")]
    NetworkErrorWasm(#[from] JsValue),

    #[error("No shared key was provided or obtained")]
    NoSharedKeyAvailable,

    #[error("No bandwidth controller provided")]
    NoBandwidthControllerAvailable,

    #[error("Connection was abruptly closed")]
    ConnectionAbruptlyClosed,

    #[error("Received response was malformed")]
    MalformedResponse,

    #[error("Credential could not be serialized")]
    SerializeCredential,

    #[error("Client is not authenticated")]
    NotAuthenticated,

    #[error("Client does not have enough bandwidth")]
    NotEnoughBandwidth,

    #[error("Received an unexpected response")]
    UnexpectedResponse,

    #[error("Connection is in an invalid state - please send a bug report")]
    ConnectionInInvalidState,

    #[error("Failed to finish registration handshake - {0}")]
    RegistrationFailure(HandshakeError),

    #[error("Authentication failure")]
    AuthenticationFailure,

    #[error("Timed out")]
    Timeout,
}

impl GatewayClientError {
    pub fn is_closed_connection(&self) -> bool {
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
