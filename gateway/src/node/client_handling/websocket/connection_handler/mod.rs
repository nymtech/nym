// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use nym_credential_verification::BandwidthFlushingBehaviourConfig;
use nym_gateway_requests::shared_key::SharedGatewayKey;
use nym_gateway_requests::ServerResponse;
use nym_sphinx::DestinationAddressBytes;
use rand::{CryptoRng, Rng};
use std::time::Duration;
use time::OffsetDateTime;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::WebSocketStream;
use tracing::{debug, instrument, trace, warn};

pub(crate) use self::authenticated::AuthenticatedHandler;
pub(crate) use self::fresh::FreshHandler;

pub(crate) mod authenticated;
mod fresh;
pub(crate) mod helpers;

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

pub(crate) struct ClientDetails {
    pub(crate) address: DestinationAddressBytes,
    pub(crate) id: i64,
    pub(crate) shared_keys: SharedGatewayKey,
    // note, this does **NOT ALWAYS** indicate timestamp of when client connected
    // it is (for v2 auth) timestamp the client **signed** when it created the request
    pub(crate) session_request_timestamp: OffsetDateTime,
}

impl ClientDetails {
    pub(crate) fn new(
        id: i64,
        address: DestinationAddressBytes,
        shared_keys: SharedGatewayKey,
        session_request_timestamp: OffsetDateTime,
    ) -> Self {
        ClientDetails {
            address,
            id,
            shared_keys,
            session_request_timestamp,
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

// imo there's no point in including the peer address in anything higher than debug
#[instrument(level = "debug", skip_all, fields(peer = %handle.peer_address))]
pub(crate) async fn handle_connection<R, S>(mut handle: FreshHandler<R, S>)
where
    R: Rng + CryptoRng + Send,
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    // don't accept any new requests if we have already received shutdown
    if handle.shutdown.is_shutdown_poll() {
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
