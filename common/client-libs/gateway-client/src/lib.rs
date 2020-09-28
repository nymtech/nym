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

use crate::error::GatewayClientError;
pub use client::GatewayClient;
pub use packet_router::{
    AcknowledgementReceiver, AcknowledgementSender, MixnetMessageReceiver, MixnetMessageSender,
};
use tungstenite::{protocol::Message, Error as WsError};

pub mod client;
pub mod error;
pub mod packet_router;
pub mod socket_state;

/// Helper method for reading from websocket stream. Helps to flatten the structure.
pub(crate) fn cleanup_socket_message(
    msg: Option<Result<Message, WsError>>,
) -> Result<Message, GatewayClientError> {
    match msg {
        Some(msg) => match msg {
            Ok(msg) => Ok(msg),
            Err(err) => Err(GatewayClientError::NetworkError(err)),
        },
        None => Err(GatewayClientError::ConnectionAbruptlyClosed),
    }
}
