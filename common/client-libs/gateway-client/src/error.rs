// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(target_arch = "wasm32")]
use crate::wasm_storage::StorageError;
#[cfg(not(target_arch = "wasm32"))]
use credential_storage::error::StorageError;
use gateway_requests::registration::handshake::error::HandshakeError;
use std::io;
use thiserror::Error;
use tungstenite::Error as WsError;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(not(feature = "coconut"))]
use web3::{contract::Error as Web3ContractError, Error as Web3Error};

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

    #[cfg(feature = "coconut")]
    #[error("Coconut error - {0}")]
    CoconutError(#[from] coconut_interface::CoconutError),

    // TODO: see if `JsValue` is a reasonable type for this
    #[cfg(target_arch = "wasm32")]
    #[error("There was a network error")]
    NetworkErrorWasm(JsValue),

    #[cfg(not(feature = "coconut"))]
    #[error("Could not burn ERC20 token in Ethereum smart contract - {0}")]
    BurnTokenError(#[from] Web3Error),

    #[cfg(not(feature = "coconut"))]
    #[error("Could not run web3 contract - {0}")]
    Web3ContractError(#[from] Web3ContractError),

    #[cfg(not(feature = "coconut"))]
    #[error("Invalid Ethereum private key")]
    InvalidEthereumPrivateKey,

    #[error("Invalid URL - {0}")]
    InvalidURL(String),

    #[error("No shared key was provided or obtained")]
    NoSharedKeyAvailable,

    #[error("No bandwidth controller provided")]
    NoBandwidthControllerAvailable,

    #[error("Credential error - {0}")]
    CredentialError(#[from] credentials::error::Error),

    #[error("Connection was abruptly closed")]
    ConnectionAbruptlyClosed,

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
