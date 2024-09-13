// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::GatewayClientError;
use nym_gateway_requests::BinaryResponse;
use tracing::warn;
use tungstenite::{protocol::Message, Error as WsError};

pub use client::{config::GatewayClientConfig, GatewayClient, GatewayConfig};
pub use nym_gateway_requests::registration::handshake::LegacySharedKeys;
pub use packet_router::{
    AcknowledgementReceiver, AcknowledgementSender, MixnetMessageReceiver, MixnetMessageSender,
    PacketRouter,
};
pub use traits::GatewayPacketRouter;

mod bandwidth;
pub mod client;
pub mod error;
pub mod packet_router;
pub mod socket_state;
pub mod traits;

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

pub(crate) fn try_decrypt_binary_message(
    bin_msg: Vec<u8>,
    shared_keys: &LegacySharedKeys,
) -> Option<Vec<u8>> {
    match BinaryResponse::try_from_encrypted_tagged_bytes(bin_msg, shared_keys) {
        Ok(bin_response) => match bin_response {
            BinaryResponse::PushedMixMessage(plaintext) => Some(plaintext),
        },
        Err(err) => {
            warn!("message received from the gateway was malformed! - {err}",);
            None
        }
    }
}
