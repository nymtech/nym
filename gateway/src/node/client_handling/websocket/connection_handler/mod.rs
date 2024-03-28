// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::storage::Storage;
use log::{trace, warn};
use nym_gateway_requests::registration::handshake::SharedKeys;
use nym_gateway_requests::ServerResponse;
use nym_sphinx::DestinationAddressBytes;
use nym_task::TaskClient;
use rand::{CryptoRng, Rng};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::WebSocketStream;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub(crate) use self::authenticated::AuthenticatedHandler;
pub(crate) use self::fresh::FreshHandler;

pub(crate) mod authenticated;
pub(crate) mod coconut;
mod fresh;

// TODO: note for my future self to consider the following idea:
// split the socket connection into sink and stream
// stream will be for reading explicit requests
// and sink for pumping responses AND mix traffic
// but as byproduct this might (or might not) break the clean "SocketStream" enum here

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

#[derive(Zeroize, ZeroizeOnDrop)]
pub(crate) struct ClientDetails {
    #[zeroize(skip)]
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

pub(crate) async fn handle_connection<R, S, St>(
    mut handle: FreshHandler<R, S, St>,
    mut shutdown: TaskClient,
) where
    R: Rng + CryptoRng,
    S: AsyncRead + AsyncWrite + Unpin + Send,
    St: Storage,
{
    // If the connection handler abruptly stops, we shouldn't signal global shutdown
    shutdown.mark_as_success();

    match shutdown
        .run_future(handle.perform_websocket_handshake())
        .await
    {
        None => {
            trace!("received shutdown signal while performing websocket handshake");
            return;
        }
        Some(Err(err)) => {
            warn!("Failed to complete WebSocket handshake: {err}. Stopping the handler");
            return;
        }
        _ => (),
    }

    trace!("Managed to perform websocket handshake!");

    match shutdown
        .run_future(handle.perform_initial_authentication())
        .await
    {
        None => {
            trace!("received shutdown signal while performing initial authentication");
            return;
        }
        Some(None) => {
            warn!("authentication has failed");
            return;
        }
        Some(Some(auth_handle)) => auth_handle.listen_for_requests(shutdown).await,
    }

    trace!("The handler is done!");
}
