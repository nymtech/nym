// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::networking::error::NetworkingError;
use crate::networking::message::OffchainMessage;
use crate::networking::sender::broadcast::Broadcaster;
use crate::networking::sender::ephemeral::EphemeralConnection;
use std::net::SocketAddr;
use std::time::Duration;

pub(crate) mod broadcast;
pub(crate) mod ephemeral;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ConnectionConfig {
    pub(crate) connection_timeout: Duration,
    pub(crate) response_timeout: Option<Duration>,
    pub(crate) send_timeout: Duration,
}

pub(crate) struct SendResponse {
    pub(crate) source: SocketAddr,
    pub(crate) response: Result<Option<OffchainMessage>, NetworkingError>,
}

impl SendResponse {
    pub(crate) fn new(
        source: SocketAddr,
        response: Result<Option<OffchainMessage>, NetworkingError>,
    ) -> Self {
        SendResponse { source, response }
    }
}

pub(crate) async fn send_single_message(
    address: SocketAddr,
    cfg: ConnectionConfig,
    message: &OffchainMessage,
) -> SendResponse {
    let res = EphemeralConnection::connect_and_send(address, cfg, message).await;
    SendResponse::new(address, res)
}

pub(crate) async fn broadcast_message(
    addresses: Vec<SocketAddr>,
    cfg: ConnectionConfig,
    message: &OffchainMessage,
) {
    Broadcaster::new(addresses, cfg)
        .broadcast(message.clone())
        .await
}

pub(crate) async fn broadcast_message_with_feedback(
    addresses: Vec<SocketAddr>,
    cfg: ConnectionConfig,
    message: &OffchainMessage,
) -> Vec<SendResponse> {
    Broadcaster::new(addresses, cfg)
        .broadcast_with_feedback(message.clone())
        .await
}

// NOTE: for the original purposes of DKG stateless broadcasts and one-off sends were enough,
// so I never implemented proper persistent connections
