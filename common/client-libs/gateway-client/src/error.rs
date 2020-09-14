// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use gateway_requests::registration::handshake::error::HandshakeError;
use std::fmt::{self, Error, Formatter};
use tungstenite::Error as WsError;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

#[derive(Debug)]
pub enum GatewayClientError {
    ConnectionNotEstablished,
    GatewayError(String),

    #[cfg(not(target_arch = "wasm32"))]
    NetworkError(WsError),

    // TODO: see if `JsValue` is a reasonable type for this
    #[cfg(target_arch = "wasm32")]
    NetworkError(JsValue),

    NoSharedKeyAvailable,
    ConnectionAbruptlyClosed,
    MalformedResponse,
    NotAuthenticated,
    ConnectionInInvalidState,
    RegistrationFailure(HandshakeError),
    AuthenticationFailure,
    Timeout,
}

#[cfg(not(target_arch = "wasm32"))]
impl From<WsError> for GatewayClientError {
    fn from(err: WsError) -> Self {
        GatewayClientError::NetworkError(err)
    }
}

#[cfg(target_arch = "wasm32")]
impl From<JsValue> for GatewayClientError {
    fn from(err: JsValue) -> Self {
        GatewayClientError::NetworkError(err)
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

            #[cfg(not(target_arch = "wasm32"))]
            GatewayClientError::NetworkError(err) => {
                write!(f, "there was a network error - {}", err)
            }
            #[cfg(target_arch = "wasm32")]
            GatewayClientError::NetworkError(err) => {
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
        }
    }
}
