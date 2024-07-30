// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use nym_credentials_interface::AvailableBandwidth;
use nym_gateway_requests::registration::handshake::SharedKeys;
use nym_gateway_requests::ServerResponse;
use nym_gateway_storage::Storage;
use nym_sphinx::DestinationAddressBytes;
use nym_task::TaskClient;
use rand::{CryptoRng, Rng};
use std::time::Duration;
use time::OffsetDateTime;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::WebSocketStream;
use tracing::{instrument, trace, warn};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub(crate) use self::authenticated::AuthenticatedHandler;
pub(crate) use self::fresh::FreshHandler;

pub(crate) mod authenticated;
pub(crate) mod ecash;
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
pub(crate) async fn handle_connection<R, S, St>(
    mut handle: FreshHandler<R, S, St>,
    mut shutdown: TaskClient,
) where
    R: Rng + CryptoRng,
    S: AsyncRead + AsyncWrite + Unpin + Send,
    St: Storage + Clone + 'static,
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
        Some(Err(err)) => {
            warn!("authentication has failed: {err}");
            return;
        }
        Some(Ok(auth_handle)) => auth_handle.listen_for_requests(shutdown).await,
    }

    trace!("The handler is done!");
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BandwidthFlushingBehaviourConfig {
    /// Defines maximum delay between client bandwidth information being flushed to the persistent storage.
    pub client_bandwidth_max_flushing_rate: Duration,

    /// Defines a maximum change in client bandwidth before it gets flushed to the persistent storage.
    pub client_bandwidth_max_delta_flushing_amount: i64,
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

pub(crate) struct ClientBandwidth {
    pub(crate) bandwidth: AvailableBandwidth,
    pub(crate) last_flushed: OffsetDateTime,

    /// the number of bytes the client had during the last sync.
    /// it is used to determine whether the current value should be synced with the storage
    /// by checking the delta with the known amount
    pub(crate) bytes_at_last_sync: i64,
    pub(crate) bytes_delta_since_sync: i64,
}

impl ClientBandwidth {
    pub(crate) fn new(bandwidth: AvailableBandwidth) -> ClientBandwidth {
        ClientBandwidth {
            bandwidth,
            last_flushed: OffsetDateTime::now_utc(),
            bytes_at_last_sync: bandwidth.bytes,
            bytes_delta_since_sync: 0,
        }
    }

    pub(crate) fn should_sync(&self, cfg: BandwidthFlushingBehaviourConfig) -> bool {
        if self.bytes_delta_since_sync.abs() >= cfg.client_bandwidth_max_delta_flushing_amount {
            return true;
        }

        if self.last_flushed + cfg.client_bandwidth_max_flushing_rate < OffsetDateTime::now_utc() {
            return true;
        }

        false
    }

    pub(crate) fn update_sync_data(&mut self) {
        self.last_flushed = OffsetDateTime::now_utc();
        self.bytes_at_last_sync = self.bandwidth.bytes;
        self.bytes_delta_since_sync = 0;
    }
}
