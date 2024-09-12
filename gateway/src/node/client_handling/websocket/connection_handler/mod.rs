// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use nym_credential_verification::BandwidthFlushingBehaviourConfig;
use nym_gateway_requests::registration::handshake::SharedKeys;
use nym_gateway_requests::ServerResponse;
use nym_gateway_storage::Storage;
use nym_sphinx::DestinationAddressBytes;
use rand::{CryptoRng, Rng};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::WebSocketStream;
use tracing::{debug, instrument, trace, warn};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub(crate) use self::authenticated::AuthenticatedHandler;
pub(crate) use self::fresh::FreshHandler;

pub(crate) mod authenticated;
mod fresh;

const WEBSOCKET_HANDSHAKE_TIMEOUT: Duration = Duration::from_millis(1_500);
const INITIAL_MESSAGE_TIMEOUT: Duration = Duration::from_millis(10_000);

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
    pub(crate) id: i64,
    pub(crate) shared_keys: SharedKeys,
}

impl ClientDetails {
    pub(crate) fn new(id: i64, address: DestinationAddressBytes, shared_keys: SharedKeys) -> Self {
        ClientDetails {
            address,
            id,
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

    fn new_failed(protocol_version: Option<u8>) -> Self {
        InitialAuthResult {
            client_details: None,
            server_response: ServerResponse::Authenticate {
                protocol_version,
                status: false,
                bandwidth_remaining: 0,
            },
        }
    }
}

// imo there's no point in including the peer address in anything higher than debug
#[instrument(level = "debug", skip_all, fields(peer = %handle.peer_address))]
pub(crate) async fn handle_connection<R, S, St>(mut handle: FreshHandler<R, S, St>)
where
    R: Rng + CryptoRng,
    S: AsyncRead + AsyncWrite + Unpin + Send,
    St: Storage + Clone + 'static,
{
    // don't accept any new requests if we have already received shutdown
    if handle.shutdown.is_shutdown() {
        debug!("stopping the handle as we have received a shutdown");
        return;
    }

    // If the connection handler abruptly stops, we shouldn't signal global shutdown
    handle.shutdown.disarm();

    match tokio::time::timeout(
        WEBSOCKET_HANDSHAKE_TIMEOUT,
        handle.perform_websocket_handshake(),
    )
    .await
    {
        Err(_elapsed) => {
            warn!("websocket handshake timeout");
            return;
        }
        Ok(Err(err)) => {
            debug!("failed to complete WebSocket handshake: {err}. Stopping the handler");
            return;
        }
        _ => {}
    }

    trace!("managed to perform websocket handshake!");

    let mut shutdown = handle.shutdown.clone();

    if let Some(auth_handle) = handle
        .handle_until_authenticated_or_failure(&mut shutdown)
        .await
    {
        auth_handle.listen_for_requests(shutdown).await
    }

    trace!("the handler is done!");
}

impl<'a> From<&'a Config> for BandwidthFlushingBehaviourConfig {
    fn from(value: &'a Config) -> Self {
        BandwidthFlushingBehaviourConfig {
            client_bandwidth_max_flushing_rate: value.debug.client_bandwidth_max_flushing_rate,
            client_bandwidth_max_delta_flushing_amount: value
                .debug
                .client_bandwidth_max_delta_flushing_amount,
        }
    }
}
