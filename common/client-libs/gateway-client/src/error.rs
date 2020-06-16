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

use gateway_requests::auth_token::AuthTokenConversionError;
use std::fmt::{self, Error, Formatter};
use tokio_tungstenite::tungstenite::Error as WsError;

#[derive(Debug)]
pub enum GatewayClientError {
    ConnectionNotEstablished,
    GatewayError(String),
    NetworkError(WsError),
    NoAuthTokenAvailable,
    ConnectionAbruptlyClosed,
    MalformedResponse,
    NotAuthenticated,
    ConnectionInInvalidState,
    AuthenticationFailure,
    Timeout,
}

impl From<WsError> for GatewayClientError {
    fn from(err: WsError) -> Self {
        GatewayClientError::NetworkError(err)
    }
}

impl From<AuthTokenConversionError> for GatewayClientError {
    fn from(_err: AuthTokenConversionError) -> Self {
        GatewayClientError::MalformedResponse
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
            GatewayClientError::NoAuthTokenAvailable => {
                write!(f, "no AuthToken was provided or obtained")
            }
            GatewayClientError::NotAuthenticated => write!(f, "client is not authenticated"),
            GatewayClientError::NetworkError(err) => {
                write!(f, "there was a network error - {}", err)
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
            GatewayClientError::AuthenticationFailure => write!(f, "authentication failure"),
            GatewayClientError::GatewayError(err) => {
                write!(f, "gateway returned an error response - {}", err)
            }
        }
    }
}
