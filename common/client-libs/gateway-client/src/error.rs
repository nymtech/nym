// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use gateway_requests::registration::handshake::error::HandshakeError;
use std::fmt::{self, Error, Formatter};
use std::io;
use tungstenite::Error as WsError;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

#[derive(Debug)]
pub enum GatewayClientError {
    ConnectionNotEstablished,
    GatewayError(String),
    NetworkError(WsError),

    // TODO: see if `JsValue` is a reasonable type for this
    #[cfg(target_arch = "wasm32")]
    NetworkErrorWasm(JsValue),

    NoSharedKeyAvailable,
    ConnectionAbruptlyClosed,
    MalformedResponse,
    SerializeCredential,
    NotAuthenticated,
    NotEnoughBandwidth((u32, i64, i64)),
    UnexpectedResponse,
    ConnectionInInvalidState,
    RegistrationFailure(HandshakeError),
    AuthenticationFailure,
    Timeout,
}

impl From<WsError> for GatewayClientError {
    fn from(err: WsError) -> Self {
        GatewayClientError::NetworkError(err)
    }
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

#[cfg(target_arch = "wasm32")]
impl From<JsValue> for GatewayClientError {
    fn from(err: JsValue) -> Self {
        GatewayClientError::NetworkErrorWasm(err)
    }
}

// better human readable representation of the error, mostly so that GatewayClientError
// would implement std::error::Error
impl fmt::Display for GatewayClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            GatewayClientError::ConnectionNotEstablished => {
                write!(f, "connection to the gateway is not established")
            }
            GatewayClientError::NoSharedKeyAvailable => {
                write!(f, "no shared key was provided or obtained")
            }
            GatewayClientError::NotAuthenticated => write!(f, "client is not authenticated"),

            GatewayClientError::NetworkError(err) => {
                write!(f, "there was a network error - {}", err)
            }
            #[cfg(target_arch = "wasm32")]
            GatewayClientError::NetworkErrorWasm(err) => {
                write!(f, "there was a network error - {:?}", err)
            }

            GatewayClientError::ConnectionAbruptlyClosed => {
                write!(f, "connection was abruptly closed")
            }
            GatewayClientError::Timeout => write!(f, "timed out"),
            GatewayClientError::MalformedResponse => write!(f, "received response was malformed"),
            GatewayClientError::ConnectionInInvalidState => write!(
                f,
                "connection is in an invalid state - please send a bug report"
            ),
            GatewayClientError::RegistrationFailure(handshake_err) => write!(
                f,
                "failed to finish registration handshake - {}",
                handshake_err
            ),
            GatewayClientError::AuthenticationFailure => write!(f, "authentication failure"),
            GatewayClientError::GatewayError(err) => {
                write!(f, "gateway returned an error response - {}", err)
            }
            GatewayClientError::UnexpectedResponse => write!(f, "received an unexpected response"),
            GatewayClientError::NotEnoughBandwidth((line_number, estimated, remaining)) => {
                write!(
                    f,
                    "line: {} - client does not have enough bandwidth: estimated {}, remaining: {}",
                    line_number, estimated, remaining
                )
            }
            GatewayClientError::SerializeCredential => {
                write!(f, "credential could not be serialized")
            }
        }
    }
}
