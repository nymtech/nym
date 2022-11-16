// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::GatewayClientError;
pub use client::GatewayClient;
pub use packet_router::{
    AcknowledgementReceiver, AcknowledgementSender, MixnetMessageReceiver, MixnetMessageSender,
};
use tungstenite::{protocol::Message, Error as WsError};

pub mod bandwidth;
pub mod client;
pub mod error;
pub mod packet_router;
pub mod socket_state;
#[cfg(feature = "wasm")]
mod wasm_storage;

/// Helper method for reading from websocket stream. Helps to flatten the structure.
pub(crate) fn cleanup_socket_message(
    msg: Option<Result<Message, WsError>>,
) -> Result<Message, GatewayClientError> {
    match msg {
        Some(msg) => msg.map_err(GatewayClientError::NetworkError),
        None => Err(GatewayClientError::ConnectionAbruptlyClosed),
    }
}

pub(crate) fn cleanup_socket_messages(
    msgs: Option<Vec<Result<Message, WsError>>>,
) -> Result<Vec<Message>, GatewayClientError> {
    match msgs {
        Some(msgs) => msgs
            .into_iter()
            .map(|msg| msg.map_err(GatewayClientError::NetworkError))
            .collect(),
        None => Err(GatewayClientError::ConnectionAbruptlyClosed),
    }
}
