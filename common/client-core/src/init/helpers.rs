// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ClientCoreError;
use crate::init::types::RegistrationResult;
use futures::{SinkExt, StreamExt};
use log::{debug, info, trace, warn};
use nym_crypto::asymmetric::ed25519;
use nym_gateway_client::GatewayClient;
use nym_gateway_requests::{ClientControlRequest, ServerResponse, CURRENT_PROTOCOL_VERSION};
use nym_topology::node::RoutingNode;
use nym_validator_client::client::IdentityKeyRef;
use nym_validator_client::UserAgent;
use rand::{seq::SliceRandom, Rng};
#[cfg(unix)]
use std::os::fd::RawFd;
use std::{sync::Arc, time::Duration};
use tungstenite::Message;
use url::Url;

#[cfg(not(target_arch = "wasm32"))]
use crate::init::websockets::connect_async;

use nym_topology::NodeId;
#[cfg(not(target_arch = "wasm32"))]
use tokio::net::TcpStream;
#[cfg(not(target_arch = "wasm32"))]
use tokio::time::sleep;
#[cfg(not(target_arch = "wasm32"))]
use tokio::time::Instant;
#[cfg(not(target_arch = "wasm32"))]
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
#[cfg(target_arch = "wasm32")]
use wasm_utils::websocket::JSWebsocket;
#[cfg(target_arch = "wasm32")]
use wasmtimer::std::Instant;
#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::sleep;

#[cfg(not(target_arch = "wasm32"))]
type WsConn = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[cfg(target_arch = "wasm32")]
type WsConn = JSWebsocket;

const CONCURRENT_GATEWAYS_MEASURED: usize = 20;
const MEASUREMENTS: usize = 3;

#[cfg(not(target_arch = "wasm32"))]
const CONN_TIMEOUT: Duration = Duration::from_millis(1500);
const PING_TIMEOUT: Duration = Duration::from_millis(1000);

// The abstraction that some of these helpers use
pub trait ConnectableGateway {
    fn node_id(&self) -> NodeId;
    fn identity(&self) -> ed25519::PublicKey;
    fn clients_address(&self, prefer_ipv6: bool) -> Option<String>;
    fn is_wss(&self) -> bool;
}

impl ConnectableGateway for RoutingNode {
    fn node_id(&self) -> NodeId {
        self.node_id
    }

    fn identity(&self) -> ed25519::PublicKey {
        self.identity_key
    }

    fn clients_address(&self, prefer_ipv6: bool) -> Option<String> {
        self.ws_entry_address(prefer_ipv6)
    }

    fn is_wss(&self) -> bool {
        self.entry
            .as_ref()
            .map(|e| e.clients_wss_port.is_some())
            .unwrap_or_default()
    }
}

struct GatewayWithLatency<'a, G: ConnectableGateway> {
    gateway: &'a G,
    latency: Duration,
}

impl<'a, G: ConnectableGateway> GatewayWithLatency<'a, G> {
    fn new(gateway: &'a G, latency: Duration) -> Self {
        GatewayWithLatency { gateway, latency }
    }
}

pub async fn gateways_for_init<R: Rng>(
    rng: &mut R,
    nym_apis: &[Url],
    user_agent: Option<UserAgent>,
    minimum_performance: u8,
    ignore_epoch_roles: bool,
) -> Result<Vec<RoutingNode>, ClientCoreError> {
    let nym_api = nym_apis
        .choose(rng)
        .ok_or(ClientCoreError::ListOfNymApisIsEmpty)?;
    let client = if let Some(user_agent) = user_agent {
        nym_validator_client::client::NymApiClient::new_with_user_agent(nym_api.clone(), user_agent)
    } else {
        nym_validator_client::client::NymApiClient::new(nym_api.clone())
    };

    log::debug!("Fetching list of gateways from: {nym_api}");

    let gateways = client.get_all_basic_entry_assigned_nodes().await?;
    info!("nym api reports {} gateways", gateways.len());

    log::trace!("Gateways: {:#?}", gateways);

    // filter out gateways below minimum performance and ones that could operate as a mixnode
    // (we don't want instability)
    let valid_gateways = gateways
        .iter()
        .filter(|g| ignore_epoch_roles || !g.supported_roles.mixnode)
        .filter(|g| g.performance.round_to_integer() >= minimum_performance)
        .filter_map(|gateway| gateway.try_into().ok())
        .collect::<Vec<_>>();
    log::debug!("After checking validity: {}", valid_gateways.len());
    log::trace!("Valid gateways: {:#?}", valid_gateways);

    log::info!(
        "and {} after validity and performance filtering",
        valid_gateways.len()
    );

    Ok(valid_gateways)
}

pub async fn gateways_for_init_with_protocol_validation<R: Rng>(
    rng: &mut R,
    nym_apis: &[Url],
    user_agent: Option<UserAgent>,
    minimum_performance: u8,
    ignore_epoch_roles: bool,
) -> Result<Vec<RoutingNode>, ClientCoreError> {
    // First get the initial list of gateways
    let gateways = gateways_for_init(
        rng,
        nym_apis,
        user_agent,
        minimum_performance,
        ignore_epoch_roles,
    )
    .await?;

    info!(
        "Checking protocol compatibility for {} gateways...",
        gateways.len()
    );

    // Filter out gateways with invalid protocols concurrently
    let validated_gateways = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    futures::stream::iter(&gateways)
        .for_each_concurrent(CONCURRENT_GATEWAYS_MEASURED, |gateway| async {
            let id = gateway.identity();
            trace!("validating protocol compatibility with {id}...");

            match validate_gateway_protocol(gateway).await {
                Ok(()) => {
                    debug!("{id}: protocol check successful");
                    validated_gateways.lock().await.push(gateway.clone());
                }
                Err(err) => {
                    warn!("failed to check protocol for {id}: {err}");
                }
            }
        })
        .await;

    let validated_gateways = validated_gateways.lock().await;

    info!(
        "Protocol check complete: {}/{} gateways responded successfully",
        validated_gateways.len(),
        gateways.len()
    );

    if validated_gateways.is_empty() {
        return Err(ClientCoreError::NoGatewaysWithCompatibleProtocol);
    }

    Ok(validated_gateways.clone())
}

#[cfg(not(target_arch = "wasm32"))]
async fn connect(endpoint: &str) -> Result<WsConn, ClientCoreError> {
    match tokio::time::timeout(CONN_TIMEOUT, connect_async(endpoint)).await {
        Err(_elapsed) => Err(ClientCoreError::GatewayConnectionTimeout),
        Ok(Err(conn_failure)) => Err(conn_failure),
        Ok(Ok((stream, _))) => Ok(stream),
    }
}

#[cfg(target_arch = "wasm32")]
async fn connect(endpoint: &str) -> Result<WsConn, ClientCoreError> {
    JSWebsocket::new(endpoint).map_err(|_| ClientCoreError::GatewayJsConnectionFailure)
}

async fn measure_latency<G>(gateway: &G) -> Result<GatewayWithLatency<G>, ClientCoreError>
where
    G: ConnectableGateway,
{
    let Some(addr) = gateway.clients_address(false) else {
        return Err(ClientCoreError::UnsupportedEntry {
            id: gateway.node_id(),
            identity: gateway.identity().to_string(),
        });
    };
    trace!(
        "establishing connection to {} ({addr})...",
        gateway.identity(),
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

        let timeout = sleep(PING_TIMEOUT);
        tokio::pin!(timeout);

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
            identity: gateway.identity().to_base58_string(),
        });
    }

    let sum: Duration = results.into_iter().sum();
    let avg = Duration::from_nanos(sum.as_nanos() as u64 / count);

    Ok(GatewayWithLatency::new(gateway, avg))
}

async fn validate_gateway_protocol<G>(gateway: &G) -> Result<(), ClientCoreError>
where
    G: ConnectableGateway,
{
    let Some(addr) = gateway.clients_address(false) else {
        return Err(ClientCoreError::UnsupportedEntry {
            id: gateway.node_id(),
            identity: gateway.identity().to_string(),
        });
    };

    trace!(
        "validating protocol compatibility with {} ({addr})...",
        gateway.identity(),
    );

    let mut stream = connect(&addr).await?;

    // Send protocol version request
    let protocol_request = ClientControlRequest::SupportedProtocol {};

    // Send the request as JSON text message
    stream.send(Message::from(protocol_request)).await?;

    // Wait for response with timeout
    let protocol_timeout = Duration::from_millis(2000);
    let response_future = stream.next();

    match tokio::time::timeout(protocol_timeout, response_future).await {
        Err(_) => {
            warn!("Gateway {} protocol check timed out", gateway.identity());
            Err(ClientCoreError::GatewayConnectionTimeout)
        }
        Ok(Some(Ok(Message::Text(response_text)))) => {
            // Try to deserialize the response
            let response = ServerResponse::try_from(response_text).map_err(|_| {
                ClientCoreError::GatewayClientError {
                    gateway_id: gateway.identity().to_base58_string(),
                    source: *Box::new(
                        nym_gateway_client::error::GatewayClientError::MalformedResponse,
                    ),
                }
            })?;

            match response {
                ServerResponse::SupportedProtocol { version } => {
                    debug!(
                        "Gateway {} supports protocol version {}, ours: {}",
                        gateway.identity(),
                        version,
                        CURRENT_PROTOCOL_VERSION
                    );

                    // Check protocol compatibility
                    if version > CURRENT_PROTOCOL_VERSION {
                        warn!(
                            "Gateway {} uses newer protocol version {} (client supports {}). \
                            Gateway should gracefully degrade, but consider updating your client.",
                            gateway.identity(),
                            version,
                            CURRENT_PROTOCOL_VERSION
                        );
                    }

                    trace!(
                        "Gateway {} protocol validation successful (gateway: v{}, client: v{})",
                        gateway.identity(),
                        version,
                        CURRENT_PROTOCOL_VERSION
                    );
                    Ok(())
                }
                ServerResponse::Error { message } => {
                    warn!(
                        "Gateway {} returned error during protocol check: {}",
                        gateway.identity(),
                        message
                    );
                    Err(ClientCoreError::GatewayClientError {
                        gateway_id: gateway.identity().to_base58_string(),
                        source: *Box::new(
                            nym_gateway_client::error::GatewayClientError::GatewayError(message),
                        ),
                    })
                }
                _ => {
                    warn!(
                        "Gateway {} returned unexpected response during protocol check",
                        gateway.identity()
                    );
                    Err(ClientCoreError::GatewayClientError {
                        gateway_id: gateway.identity().to_base58_string(),
                        source: *Box::new(
                            nym_gateway_client::error::GatewayClientError::UnexpectedResponse {
                                name: response.name().to_string(),
                            },
                        ),
                    })
                }
            }
        }
        Ok(Some(Ok(_))) => {
            warn!(
                "Gateway {} sent non-text response during protocol check",
                gateway.identity()
            );
            Err(ClientCoreError::GatewayConnectionAbruptlyClosed)
        }
        Ok(Some(Err(e))) => {
            warn!(
                "WebSocket error during protocol check with {}: {}",
                gateway.identity(),
                e
            );
            Err(e.into())
        }
        Ok(None) => {
            warn!(
                "Gateway {} closed connection during protocol check",
                gateway.identity()
            );
            Err(ClientCoreError::GatewayConnectionAbruptlyClosed)
        }
    }
}

pub async fn choose_gateway_by_latency<R: Rng, G: ConnectableGateway + Clone>(
    rng: &mut R,
    gateways: &[G],
    must_use_tls: bool,
) -> Result<G, ClientCoreError> {
    let gateways = filter_by_tls(gateways, must_use_tls)?;

    info!(
        "choosing gateway by latency, pinging {} gateways ...",
        gateways.len()
    );

    let gateways_with_latency = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    futures::stream::iter(gateways)
        .for_each_concurrent(CONCURRENT_GATEWAYS_MEASURED, |gateway| async {
            let id = gateway.identity();
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
        chosen.gateway.identity(),
        chosen.latency
    );

    Ok(chosen.gateway.clone())
}

fn filter_by_tls<G: ConnectableGateway>(
    gateways: &[G],
    must_use_tls: bool,
) -> Result<Vec<&G>, ClientCoreError> {
    if must_use_tls {
        let filtered = gateways.iter().filter(|g| g.is_wss()).collect::<Vec<_>>();

        if filtered.is_empty() {
            return Err(ClientCoreError::NoWssGateways);
        }

        Ok(filtered)
    } else {
        Ok(gateways.iter().collect())
    }
}

pub(super) fn uniformly_random_gateway<R: Rng>(
    rng: &mut R,
    gateways: &[RoutingNode],
    must_use_tls: bool,
) -> Result<RoutingNode, ClientCoreError> {
    filter_by_tls(gateways, must_use_tls)?
        .choose(rng)
        .ok_or(ClientCoreError::NoGatewaysOnNetwork)
        .map(|&r| r.clone())
}

pub(super) fn get_specified_gateway(
    gateway_identity: IdentityKeyRef,
    gateways: &[RoutingNode],
    must_use_tls: bool,
) -> Result<RoutingNode, ClientCoreError> {
    log::debug!("Requesting specified gateway: {}", gateway_identity);
    let user_gateway = ed25519::PublicKey::from_base58_string(gateway_identity)
        .map_err(ClientCoreError::UnableToCreatePublicKeyFromGatewayId)?;

    let gateway = gateways
        .iter()
        .find(|gateway| gateway.identity_key == user_gateway)
        .ok_or_else(|| ClientCoreError::NoGatewayWithId(gateway_identity.to_string()))?;

    let Some(entry_details) = gateway.entry.as_ref() else {
        return Err(ClientCoreError::UnsupportedEntry {
            id: gateway.node_id,
            identity: gateway.identity().to_string(),
        });
    };

    if must_use_tls && entry_details.clients_wss_port.is_none() {
        return Err(ClientCoreError::UnsupportedWssProtocol {
            gateway: gateway_identity.to_string(),
        });
    }

    Ok(gateway.clone())
}

pub(super) async fn register_with_gateway(
    gateway_id: ed25519::PublicKey,
    gateway_listener: Url,
    our_identity: Arc<ed25519::KeyPair>,
    #[cfg(unix)] connection_fd_callback: Option<Arc<dyn Fn(RawFd) + Send + Sync>>,
) -> Result<RegistrationResult, ClientCoreError> {
    let mut gateway_client = GatewayClient::new_init(
        gateway_listener,
        gateway_id,
        our_identity.clone(),
        #[cfg(unix)]
        connection_fd_callback,
    );

    gateway_client.establish_connection().await.map_err(|err| {
        log::warn!("Failed to establish connection with gateway!");
        ClientCoreError::GatewayClientError {
            gateway_id: gateway_id.to_base58_string(),
            source: err,
        }
    })?;
    let auth_response = gateway_client
        .perform_initial_authentication()
        .await
        .map_err(|err| {
            log::warn!("Failed to register with the gateway {gateway_id}: {err}");
            ClientCoreError::GatewayClientError {
                gateway_id: gateway_id.to_base58_string(),
                source: err,
            }
        })?;

    // this should NEVER happen, if it did, it means the function was misused,
    // because for any fresh **registration**, the derived key is always up to date
    if auth_response.requires_key_upgrade {
        return Err(ClientCoreError::UnexpectedKeyUpgrade {
            gateway_id: gateway_id.to_base58_string(),
        });
    }

    Ok(RegistrationResult {
        shared_keys: auth_response.initial_shared_key,
        authenticated_ephemeral_client: gateway_client,
    })
}
