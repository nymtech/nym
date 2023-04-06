// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::key_manager::KeyManager,
    config::{persistence::key_pathfinder::ClientKeyPathfinder, Config},
    error::ClientCoreError,
};
use futures::{SinkExt, StreamExt};
use log::{debug, info, trace, warn};
use nym_config::NymConfig;
use nym_crypto::asymmetric::identity;
use nym_gateway_client::GatewayClient;
use nym_gateway_requests::registration::handshake::SharedKeys;
use nym_topology::{filter::VersionFilterable, gateway};
use rand::{seq::SliceRandom, thread_rng, Rng};
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
use nym_gateway_client::wasm_mockups::DirectSigningNyxdClient;
#[cfg(target_arch = "wasm32")]
use wasm_timer::Instant;
#[cfg(target_arch = "wasm32")]
use wasm_utils::websocket::JSWebsocket;

#[cfg(target_arch = "wasm32")]
type WsConn = JSWebsocket;

const MEASUREMENTS: usize = 3;

#[cfg(not(target_arch = "wasm32"))]
const CONN_TIMEOUT: Duration = Duration::from_millis(1500);
const PING_TIMEOUT: Duration = Duration::from_millis(1000);

struct GatewayWithLatency {
    gateway: gateway::Node,
    latency: Duration,
}

impl GatewayWithLatency {
    fn new(gateway: gateway::Node, latency: Duration) -> Self {
        GatewayWithLatency { gateway, latency }
    }
}

async fn current_gateways<R: Rng>(
    rng: &mut R,
    nym_apis: Vec<Url>,
) -> Result<Vec<gateway::Node>, ClientCoreError> {
    let nym_api = nym_apis
        .choose(rng)
        .ok_or(ClientCoreError::ListOfNymApisIsEmpty)?;
    let client = nym_validator_client::client::NymApiClient::new(nym_api.clone());

    log::trace!("Fetching list of gateways from: {}", nym_api);

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

async fn measure_latency(gateway: gateway::Node) -> Result<GatewayWithLatency, ClientCoreError> {
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

async fn choose_gateway_by_latency<R: Rng>(
    rng: &mut R,
    gateways: Vec<gateway::Node>,
) -> Result<gateway::Node, ClientCoreError> {
    info!("choosing gateway by latency...");

    let mut gateways_with_latency = Vec::new();
    for gateway in gateways {
        let id = *gateway.identity();
        trace!("measuring latency to {id}...");
        let with_latency = match measure_latency(gateway).await {
            Ok(res) => res,
            Err(err) => {
                warn!("failed to measure {id}: {err}");
                continue;
            }
        };
        debug!("{id}: {:?}", with_latency.latency);
        gateways_with_latency.push(with_latency)
    }

    let chosen = gateways_with_latency
        .choose_weighted(rng, |item| 1. / item.latency.as_secs_f32())
        .expect("invalid selection weight!");

    info!(
        "chose gateway {} with average latency of {:?}",
        chosen.gateway.identity_key, chosen.latency
    );

    Ok(chosen.gateway.clone())
}

fn uniformly_random_gateway<R: Rng>(
    rng: &mut R,
    gateways: Vec<gateway::Node>,
) -> Result<gateway::Node, ClientCoreError> {
    gateways
        .choose(rng)
        .ok_or(ClientCoreError::NoGatewaysOnNetwork)
        .cloned()
}

pub(super) async fn query_gateway_details(
    validator_servers: Vec<Url>,
    chosen_gateway_id: Option<identity::PublicKey>,
    by_latency: bool,
) -> Result<gateway::Node, ClientCoreError> {
    let mut rng = thread_rng();
    let gateways = current_gateways(&mut rng, validator_servers).await?;

    // if we set an explicit gateway, use that one and nothing else
    if let Some(explicitly_chosen) = chosen_gateway_id {
        gateways
            .into_iter()
            .find(|gateway| gateway.identity_key == explicitly_chosen)
            .ok_or_else(|| ClientCoreError::NoGatewayWithId(explicitly_chosen.to_string()))
    } else if by_latency {
        choose_gateway_by_latency(&mut rng, gateways).await
    } else {
        uniformly_random_gateway(&mut rng, gateways)
    }
}

pub(super) async fn register_with_gateway(
    gateway: &gateway::Node,
    our_identity: Arc<identity::KeyPair>,
) -> Result<Arc<SharedKeys>, ClientCoreError> {
    let timeout = Duration::from_millis(1500);
    let mut gateway_client: GatewayClient<DirectSigningNyxdClient> = GatewayClient::new_init(
        gateway.clients_address(),
        gateway.identity_key,
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

pub(super) fn store_keys<T>(
    key_manager: &KeyManager,
    config: &Config<T>,
) -> Result<(), ClientCoreError>
where
    T: NymConfig,
{
    let pathfinder = ClientKeyPathfinder::new_from_config(config);
    Ok(key_manager
        .store_keys(&pathfinder)
        .tap_err(|err| log::error!("Failed to generate keys: {err}"))?)
}
