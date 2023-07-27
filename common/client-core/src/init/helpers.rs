// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::GatewayEndpointConfig;
use crate::error::ClientCoreError;
use futures::{SinkExt, StreamExt};
use log::{debug, info, trace, warn};
use nym_crypto::asymmetric::identity;
use nym_gateway_client::GatewayClient;
use nym_gateway_requests::registration::handshake::SharedKeys;
use nym_topology::{filter::VersionFilterable, gateway};
use rand::{seq::SliceRandom, Rng};
use std::{sync::Arc, time::Duration};
use tap::TapFallible;
use tungstenite::Message;
use url::Url;

#[cfg(not(target_arch = "wasm32"))]
use nym_validator_client::nyxd::DirectSigningNyxdClient;
#[cfg(not(target_arch = "wasm32"))]
use tokio::net::TcpStream;
#[cfg(not(target_arch = "wasm32"))]
use tokio::time::Instant;
#[cfg(not(target_arch = "wasm32"))]
use tokio_tungstenite::connect_async;
#[cfg(not(target_arch = "wasm32"))]
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
#[cfg(not(target_arch = "wasm32"))]
type WsConn = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[cfg(target_arch = "wasm32")]
use nym_bandwidth_controller::wasm_mockups::DirectSigningNyxdClient;
#[cfg(target_arch = "wasm32")]
use wasm_timer::Instant;
#[cfg(target_arch = "wasm32")]
use wasm_utils::websocket::JSWebsocket;

#[cfg(target_arch = "wasm32")]
type WsConn = JSWebsocket;

const CONCURRENT_GATEWAYS_MEASURED: usize = 20;
const MEASUREMENTS: usize = 3;

#[cfg(not(target_arch = "wasm32"))]
const CONN_TIMEOUT: Duration = Duration::from_millis(1500);
const PING_TIMEOUT: Duration = Duration::from_millis(1000);

struct GatewayWithLatency<'a> {
    gateway: &'a gateway::Node,
    latency: Duration,
}

impl<'a> GatewayWithLatency<'a> {
    fn new(gateway: &'a gateway::Node, latency: Duration) -> Self {
        GatewayWithLatency { gateway, latency }
    }
}

pub async fn current_gateways<R: Rng>(
    rng: &mut R,
    nym_apis: &[Url],
) -> Result<Vec<gateway::Node>, ClientCoreError> {
    let nym_api = nym_apis
        .choose(rng)
        .ok_or(ClientCoreError::ListOfNymApisIsEmpty)?;
    let client = nym_validator_client::client::NymApiClient::new(nym_api.clone());

    log::trace!("Fetching list of gateways from: {nym_api}");

    let gateways = client.get_cached_gateways().await?;
    let valid_gateways = gateways
        .into_iter()
        .filter_map(|gateway| gateway.try_into().ok())
        .collect::<Vec<gateway::Node>>();

    // we were always filtering by version so I'm not removing that 'feature'
    let filtered_gateways = valid_gateways.filter_by_version(env!("CARGO_PKG_VERSION"));
    Ok(filtered_gateways)
}

#[cfg(not(target_arch = "wasm32"))]
async fn connect(endpoint: &str) -> Result<WsConn, ClientCoreError> {
    match tokio::time::timeout(CONN_TIMEOUT, connect_async(endpoint)).await {
        Err(_elapsed) => Err(ClientCoreError::GatewayConnectionTimeout),
        Ok(Err(conn_failure)) => Err(conn_failure.into()),
        Ok(Ok((stream, _))) => Ok(stream),
    }
}

#[cfg(target_arch = "wasm32")]
async fn connect(endpoint: &str) -> Result<WsConn, ClientCoreError> {
    JSWebsocket::new(endpoint).map_err(|_| ClientCoreError::GatewayJsConnectionFailure)
}

async fn measure_latency(gateway: &gateway::Node) -> Result<GatewayWithLatency, ClientCoreError> {
    let addr = gateway.clients_address();
    trace!(
        "establishing connection to {} ({addr})...",
        gateway.identity_key,
    );
    let mut stream = connect(&addr).await?;

    let mut results = Vec::new();
    for _ in 0..MEASUREMENTS {
        let measurement_future = async {
            let ping_content = vec![1, 2, 3];
            let start = Instant::now();
            stream.send(Message::Ping(ping_content.clone())).await?;

            match stream.next().await {
                Some(Ok(Message::Pong(content))) => {
                    if content == ping_content {
                        let elapsed = Instant::now().duration_since(start);
                        trace!("current ping time: {elapsed:?}");
                        results.push(elapsed);
                    } else {
                        warn!("received a pong message with different content? wtf.")
                    }
                }
                Some(Ok(_)) => warn!("received a message that's not a pong!"),
                Some(Err(err)) => return Err(err.into()),
                None => return Err(ClientCoreError::GatewayConnectionAbruptlyClosed),
            }

            Ok::<(), ClientCoreError>(())
        };

        // thanks to wasm we can't use tokio::time::timeout : (
        #[cfg(not(target_arch = "wasm32"))]
        let timeout = tokio::time::sleep(PING_TIMEOUT);
        #[cfg(not(target_arch = "wasm32"))]
        tokio::pin!(timeout);

        #[cfg(target_arch = "wasm32")]
        let mut timeout = wasm_timer::Delay::new(PING_TIMEOUT);

        tokio::select! {
            _ = &mut timeout => {
                warn!("timed out while trying to perform measurement...")
            }
            res = measurement_future => res?,
        }
    }

    let count = results.len() as u64;
    if count == 0 {
        return Err(ClientCoreError::NoGatewayMeasurements {
            identity: gateway.identity_key.to_base58_string(),
        });
    }

    let sum: Duration = results.into_iter().sum();
    let avg = Duration::from_nanos(sum.as_nanos() as u64 / count);

    Ok(GatewayWithLatency::new(gateway, avg))
}

pub(super) async fn choose_gateway_by_latency<R: Rng>(
    rng: &mut R,
    gateways: &[gateway::Node],
) -> Result<gateway::Node, ClientCoreError> {
    info!(
        "choosing gateway by latency, pinging {} gateways ...",
        gateways.len()
    );

    let gateways_with_latency = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    futures::stream::iter(gateways)
        .for_each_concurrent(CONCURRENT_GATEWAYS_MEASURED, |gateway| async {
            let id = *gateway.identity();
            trace!("measuring latency to {id}...");
            match measure_latency(gateway).await {
                Ok(with_latency) => {
                    debug!("{id}: {:?}", with_latency.latency);
                    gateways_with_latency.lock().await.push(with_latency);
                }
                Err(err) => {
                    warn!("failed to measure {id}: {err}");
                }
            };
        })
        .await;

    let gateways_with_latency = gateways_with_latency.lock().await;
    let chosen = gateways_with_latency
        .choose_weighted(rng, |item| 1. / item.latency.as_secs_f32())
        .expect("invalid selection weight!");

    info!(
        "chose gateway {} with average latency of {:?}",
        chosen.gateway.identity_key, chosen.latency
    );

    Ok(chosen.gateway.clone())
}

pub(super) fn uniformly_random_gateway<R: Rng>(
    rng: &mut R,
    gateways: &[gateway::Node],
) -> Result<gateway::Node, ClientCoreError> {
    gateways
        .choose(rng)
        .ok_or(ClientCoreError::NoGatewaysOnNetwork)
        .cloned()
}

pub(super) async fn register_with_gateway(
    gateway: &GatewayEndpointConfig,
    our_identity: Arc<identity::KeyPair>,
) -> Result<Arc<SharedKeys>, ClientCoreError> {
    let timeout = Duration::from_millis(1500);
    let mut gateway_client: GatewayClient<DirectSigningNyxdClient, _> = GatewayClient::new_init(
        gateway.gateway_listener.clone(),
        gateway.try_get_gateway_identity_key()?,
        our_identity.clone(),
        timeout,
    );
    gateway_client
        .establish_connection()
        .await
        .tap_err(|_| log::warn!("Failed to establish connection with gateway!"))?;
    let shared_keys = gateway_client
        .perform_initial_authentication()
        .await
        .tap_err(|_| log::warn!("Failed to register with the gateway!"))?;
    Ok(shared_keys)
}
