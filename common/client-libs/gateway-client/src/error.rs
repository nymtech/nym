// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(target_arch = "wasm32")]
use crate::wasm_mockups::StorageError;
use gateway_requests::registration::handshake::error::HandshakeError;
#[cfg(feature = "mobile")]
#[cfg(not(target_arch = "wasm32"))]
use mobile_storage::StorageError;
#[cfg(not(feature = "mobile"))]
#[cfg(not(target_arch = "wasm32"))]
use nym_credential_storage::error::StorageError;
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

    #[error("There was a credential storage error - {0}")]
    CredentialStorageError(#[from] StorageError),

    #[error("Coconut error - {0}")]
    CoconutError(#[from] nym_coconut_interface::CoconutError),

    // TODO: see if `JsValue` is a reasonable type for this
    #[cfg(target_arch = "wasm32")]
    #[error("There was a network error")]
    NetworkErrorWasm(JsValue),

    #[error("Invalid URL - {0}")]
    InvalidURL(String),

    #[error("No shared key was provided or obtained")]
    NoSharedKeyAvailable,

    #[error("No bandwidth controller provided")]
    NoBandwidthControllerAvailable,

    #[error("Credential error - {0}")]
    CredentialError(#[from] nym_credentials::error::Error),

    #[error("Connection was abruptly closed")]
    ConnectionAbruptlyClosed,

    #[error("Connection was abruptly closed as gateway was stopped")]
    ConnectionClosedGatewayShutdown,

    #[error("Received response was malformed")]
    MalformedResponse,

    #[error("Credential could not be serialized")]
    SerializeCredential,

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

    #[error("Failed to finish registration handshake - {0}")]
    RegistrationFailure(HandshakeError),

    #[error("Authentication failure")]
    AuthenticationFailure,

    #[error("Timed out")]
    Timeout,

    #[error("Failed to send mixnet message")]
    MixnetMsgSenderFailedToSend,

    #[error("Attempted to negotiate connection with gateway using incompatible protocol version. Ours is {current} and the gateway reports {gateway:?}")]
    IncompatibleProtocol { gateway: Option<u8>, current: u8 },
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
