// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use gateway_requests::registration::handshake::SharedKeys;
use gateway_requests::ServerResponse;
use log::{trace, warn};
use nymsphinx::DestinationAddressBytes;
use rand::{CryptoRng, Rng};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::WebSocketStream;

pub(crate) use self::authenticated::AuthenticatedHandler;
pub(crate) use self::fresh::FreshHandler;

mod authenticated;
#[cfg(not(feature = "coconut"))]
mod eth_events;
mod fresh;

//// TODO: note for my future self to consider the following idea:
//// split the socket connection into sink and stream
//// stream will be for reading explicit requests
//// and sink for pumping responses AND mix traffic
//// but as byproduct this might (or might not) break the clean "SocketStream" enum here

pub(crate) enum SocketStream<S> {
    RawTcp(S),
    UpgradedWebSocket(WebSocketStream<S>),
    Invalid,
}

impl<S> SocketStream<S> {
    fn is_websocket(&self) -> bool {
        matches!(self, SocketStream::UpgradedWebSocket(_))
    }
}

#[derive(Copy, Clone)]
pub(crate) struct ClientDetails {
    pub(crate) address: DestinationAddressBytes,
    pub(crate) shared_keys: SharedKeys,
}

impl ClientDetails {
    pub(crate) fn new(address: DestinationAddressBytes, shared_keys: SharedKeys) -> Self {
        ClientDetails {
            address,
            shared_keys,
        }
    }
}

pub(crate) struct InitialAuthResult {
    pub(crate) client_details: Option<ClientDetails>,
    pub(crate) server_response: ServerResponse,
}

impl InitialAuthResult {
    fn new(client_details: Option<ClientDetails>, server_response: ServerResponse) -> Self {
        InitialAuthResult {
            client_details,
            server_response,
        }
    }
}

pub(crate) async fn handle_connection<R, S>(mut handle: FreshHandler<R, S>)
where
    R: Rng + CryptoRng,
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    if let Err(err) = handle.perform_websocket_handshake().await {
        warn!(
            "Failed to complete WebSocket handshake - {}. Stopping the handler",
            err
        );
        return;
    }

    trace!("Managed to perform websocket handshake!");

    if let Some(auth_handle) = handle.perform_initial_authentication().await {
        auth_handle.listen_for_requests().await
    } else {
        warn!("Authentication has failed")
    }
    trace!("The handler is done!");
}
