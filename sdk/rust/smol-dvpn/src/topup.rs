// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Background bandwidth monitoring and top-up for a long-lived tunnel.
//!
//! A tunnel's registered bandwidth is finite. This module polls the gateway
//! `metadata` endpoint's `available_bandwidth` **through the tunnel itself**
//! (via the tunnel's [`TunnelConnector`], never the host network, so the
//! client's real IP is never exposed to the metadata endpoint) and:
//!
//! - emits [`BandwidthEvent`]s so an implementer can react (e.g. prompt the user
//!   to buy more ticketbooks), and
//! - when a [`BandwidthCredentialSource`] is configured, spends one stored ticket
//!   to extend the bandwidth before it is exhausted.
//!
//! Monitoring runs whenever a metadata endpoint is known, independently of
//! whether automatic top-up is enabled — so the events are available even in a
//! monitor-only configuration.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_credentials_interface::{BandwidthCredential, TicketType};
use nym_crypto::asymmetric::ed25519;
use nym_http_api_client::Client as MetadataHttpClient;
use nym_wireguard_private_metadata_client::WireguardMetadataApiClient;
use nym_wireguard_private_metadata_shared::interface::{RequestData, ResponseData};
use nym_wireguard_private_metadata_shared::{
    routes, Construct, Extract, Request, Response, Version,
};
use time::OffsetDateTime;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::connectors::TunnelConnector;
use crate::error::{DvpnError, Result};

/// Metadata protocol version this client speaks.
const PROTOCOL_VERSION: Version = Version::V2;
/// Default low-water mark: top up when under 100 MiB remains.
const DEFAULT_THRESHOLD_BYTES: i64 = 100 * 1024 * 1024;
/// Default poll cadence.
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(60);
/// Broadcast channel depth for bandwidth events.
const EVENT_CHANNEL_CAPACITY: usize = 16;

/// Future returned by a [`BandwidthCredentialSource`].
pub type CredentialFuture<'a> =
    Pin<Box<dyn Future<Output = Result<BandwidthCredential>> + Send + 'a>>;

/// Supplies bandwidth credentials on demand — one spent ticket per call.
pub trait BandwidthCredentialSource: Send + Sync {
    /// Obtain a fresh spendable credential (spends one stored ticket).
    fn obtain(&self) -> CredentialFuture<'_>;
}

/// A ready-made [`BandwidthCredentialSource`] backed by any
/// [`BandwidthTicketProvider`] (e.g. `nym-sdk-session`'s controller sender).
/// Spends one stored ticket of `ticket_type` against `gateway_id` per call.
pub struct ProviderCredentialSource {
    provider: Arc<dyn BandwidthTicketProvider>,
    gateway_id: ed25519::PublicKey,
    ticket_type: TicketType,
}

impl ProviderCredentialSource {
    /// Build a source that spends `ticket_type` tickets for `gateway_id` via `provider`.
    pub fn new(
        provider: Arc<dyn BandwidthTicketProvider>,
        gateway_id: ed25519::PublicKey,
        ticket_type: TicketType,
    ) -> Self {
        Self {
            provider,
            gateway_id,
            ticket_type,
        }
    }
}

impl BandwidthCredentialSource for ProviderCredentialSource {
    fn obtain(&self) -> CredentialFuture<'_> {
        Box::pin(async move {
            let prepared = self
                .provider
                .get_ecash_ticket(
                    self.ticket_type,
                    self.gateway_id,
                    1,
                    OffsetDateTime::now_utc(),
                )
                .await
                .map_err(|e| DvpnError::Transport(format!("obtain credential: {e}")))?
                .ok_or_else(|| {
                    DvpnError::Transport("no stored ticket available for top-up".into())
                })?;
            Ok(BandwidthCredential::from(prepared.data))
        })
    }
}

/// An event emitted by the bandwidth monitor/top-up task. Subscribe via
/// [`Tunnel::bandwidth_events`](crate::Tunnel::bandwidth_events).
#[derive(Clone, Debug)]
pub enum BandwidthEvent {
    /// Available bandwidth dropped below the configured threshold.
    Low {
        /// Bytes currently available.
        available: i64,
        /// The configured low-water threshold in bytes.
        threshold: i64,
    },
    /// A top-up succeeded; carries the new available bandwidth in bytes.
    ToppedUp {
        /// Bytes available after the top-up.
        new_available: i64,
    },
    /// A top-up attempt failed (obtaining a credential or spending it).
    TopupFailed {
        /// Human-readable failure reason.
        reason: String,
    },
    /// Available bandwidth reached zero (or below).
    Exhausted,
}

/// Configuration for the background top-up / monitoring task.
#[derive(Clone, Debug)]
pub struct TopupConfig {
    /// Base URL of the gateway `metadata` HTTP endpoint, reachable in-tunnel.
    pub metadata_url: String,
    /// Emit a `Low` event (and top up, if enabled) when available bandwidth
    /// drops below this many bytes.
    pub threshold_bytes: i64,
    /// How often to poll available bandwidth.
    pub poll_interval: Duration,
}

impl TopupConfig {
    /// A config with default threshold/cadence for the given metadata endpoint.
    pub fn new(metadata_url: impl Into<String>) -> Self {
        Self {
            metadata_url: metadata_url.into(),
            threshold_bytes: DEFAULT_THRESHOLD_BYTES,
            poll_interval: DEFAULT_POLL_INTERVAL,
        }
    }
}

fn build_request(data: RequestData) -> Result<Request> {
    <Request as Construct<RequestData>>::construct(data, PROTOCOL_VERSION)
        .map_err(|e| DvpnError::Transport(format!("build metadata request: {e}")))
}

fn extract_response(resp: &Response) -> Result<ResponseData> {
    <_ as Extract<ResponseData>>::extract(resp)
        .map(|(data, _version)| data)
        .map_err(|e| DvpnError::Transport(format!("decode metadata response: {e}")))
}

/// A metadata endpoint client that dials **through the tunnel** using the
/// tunnel's [`TunnelConnector`], so top-up traffic never leaves via the host
/// network. Uses a fresh in-tunnel HTTP/1 connection per request (requests are
/// infrequent — one poll interval apart).
pub(crate) struct TunnelMetadataClient {
    connector: TunnelConnector,
    base_url: String,
}

impl TunnelMetadataClient {
    pub(crate) fn new(connector: TunnelConnector, base_url: String) -> Self {
        Self {
            connector,
            base_url,
        }
    }

    async fn post(&self, segments: &[&str], body: &Request) -> Result<Response> {
        use tower::Service as _;

        let uri: http::Uri = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            segments.join("/")
        )
        .parse()
        .map_err(|e| DvpnError::Config(format!("invalid metadata url: {e}")))?;
        let authority = uri
            .authority()
            .map(|a| a.as_str().to_string())
            .ok_or_else(|| DvpnError::Config("metadata url has no host".into()))?;
        let path_and_query = uri
            .path_and_query()
            .map(|p| p.as_str().to_string())
            .unwrap_or_else(|| uri.path().to_string());

        let payload = serde_json::to_vec(body)
            .map_err(|e| DvpnError::Transport(format!("encode metadata request: {e}")))?;

        // Open an in-tunnel TCP connection to the metadata endpoint and speak HTTP/1 over it.
        let mut connector = self.connector.clone();
        let io = connector.call(uri.clone()).await?;
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
            .await
            .map_err(|e| DvpnError::Transport(format!("metadata connection handshake: {e}")))?;
        // Drive the connection while we send the request and read the response.
        let conn_task = tokio::spawn(async move {
            let _ = conn.await;
        });

        let request = http::Request::builder()
            .method(http::Method::POST)
            .uri(path_and_query)
            .header(http::header::HOST, authority)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(payload)))
            .map_err(|e| DvpnError::Transport(format!("build metadata request: {e}")))?;

        let resp = sender
            .send_request(request)
            .await
            .map_err(|e| DvpnError::Transport(format!("metadata request: {e}")))?;
        let status = resp.status();
        let bytes = resp
            .into_body()
            .collect()
            .await
            .map_err(|e| DvpnError::Transport(format!("read metadata response: {e}")))?
            .to_bytes();
        conn_task.abort();

        if !status.is_success() {
            return Err(DvpnError::Transport(format!(
                "metadata endpoint returned HTTP {status}"
            )));
        }
        serde_json::from_slice(&bytes)
            .map_err(|e| DvpnError::Transport(format!("decode metadata response: {e}")))
    }

    /// Query the gateway for currently available bandwidth (bytes), in-tunnel.
    pub(crate) async fn available_bandwidth(&self) -> Result<i64> {
        let req = build_request(RequestData::AvailableBandwidth)?;
        let resp = self
            .post(
                &[routes::V1_API_VERSION, routes::BANDWIDTH, routes::AVAILABLE],
                &req,
            )
            .await?;
        match extract_response(&resp)? {
            ResponseData::AvailableBandwidth { amount, .. } => Ok(amount),
            _ => Err(DvpnError::Transport(
                "unexpected response to available-bandwidth query".into(),
            )),
        }
    }

    /// Spend one credential, returning the updated available bandwidth (bytes), in-tunnel.
    pub(crate) async fn topup(&self, credential: BandwidthCredential) -> Result<i64> {
        let req = build_request(RequestData::TopUpBandwidth {
            credential: Box::new(credential),
        })?;
        let resp = self
            .post(
                &[routes::V1_API_VERSION, routes::BANDWIDTH, routes::TOPUP],
                &req,
            )
            .await?;
        match extract_response(&resp)? {
            ResponseData::TopUpBandwidth {
                available_bandwidth,
                ..
            } => Ok(available_bandwidth),
            _ => Err(DvpnError::Transport(
                "unexpected response to top-up request".into(),
            )),
        }
    }
}

/// The background monitor/top-up task: poll available bandwidth through the
/// tunnel, emit [`BandwidthEvent`]s, and (when `source` is set) top up before the
/// bandwidth is exhausted. Runs until the `CancellationToken` fires.
pub(crate) async fn run_topup(
    config: TopupConfig,
    source: Option<Arc<dyn BandwidthCredentialSource>>,
    connector: TunnelConnector,
    events: broadcast::Sender<BandwidthEvent>,
    cancel: CancellationToken,
) {
    let client = TunnelMetadataClient::new(connector, config.metadata_url.clone());

    let mut ticker = tokio::time::interval(config.poll_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                debug!("bandwidth monitor task cancelled");
                break;
            }
            _ = ticker.tick() => {
                match client.available_bandwidth().await {
                    Ok(avail) => {
                        if avail <= 0 {
                            let _ = events.send(BandwidthEvent::Exhausted);
                        }
                        if avail < config.threshold_bytes {
                            let _ = events.send(BandwidthEvent::Low {
                                available: avail,
                                threshold: config.threshold_bytes,
                            });
                            if let Some(source) = &source {
                                info!(available = avail, threshold = config.threshold_bytes,
                                      "bandwidth low; topping up");
                                match source.obtain().await {
                                    Ok(cred) => match client.topup(cred).await {
                                        Ok(new_avail) => {
                                            info!(available = new_avail, "topped up");
                                            let _ = events.send(BandwidthEvent::ToppedUp {
                                                new_available: new_avail,
                                            });
                                        }
                                        Err(e) => {
                                            warn!("top-up failed: {e}");
                                            let _ = events.send(BandwidthEvent::TopupFailed {
                                                reason: e.to_string(),
                                            });
                                        }
                                    },
                                    Err(e) => {
                                        warn!("could not obtain credential for top-up: {e}");
                                        let _ = events.send(BandwidthEvent::TopupFailed {
                                            reason: e.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => debug!("available-bandwidth query failed: {e}"),
                }
            }
        }
    }
}

/// Create a broadcast channel for [`BandwidthEvent`]s.
pub(crate) fn event_channel() -> broadcast::Sender<BandwidthEvent> {
    broadcast::Sender::new(EVENT_CHANNEL_CAPACITY)
}

// ---------------------------------------------------------------------------
// Host-network one-shot diagnostics
//
// These helpers talk to a metadata endpoint over the HOST network (not through a
// tunnel). They exist for probing a directly-reachable endpoint from tooling and
// examples; the tunnel's own background top-up always goes in-tunnel (see
// `run_topup` above). Do not use these for a live tunnel's top-up.
// ---------------------------------------------------------------------------

/// Build a host-network metadata HTTP client for the given endpoint URL.
fn host_metadata_client(url: &str) -> Result<MetadataHttpClient> {
    MetadataHttpClient::new_url(url, None)
        .map_err(|e| DvpnError::Config(format!("invalid metadata url {url}: {e}")))
}

/// One-shot (host network): query available bandwidth (bytes) at a metadata endpoint.
pub async fn query_available_bandwidth(metadata_url: &str) -> Result<i64> {
    let client = host_metadata_client(metadata_url)?;
    let req = build_request(RequestData::AvailableBandwidth)?;
    let resp = client
        .available_bandwidth(&req)
        .await
        .map_err(|e| DvpnError::Transport(format!("available_bandwidth: {e}")))?;
    match extract_response(&resp)? {
        ResponseData::AvailableBandwidth { amount, .. } => Ok(amount),
        _ => Err(DvpnError::Transport(
            "unexpected response to available-bandwidth query".into(),
        )),
    }
}

/// One-shot (host network): spend `credential` at a metadata endpoint, returning the
/// updated available bandwidth (bytes).
pub async fn topup_bandwidth(metadata_url: &str, credential: BandwidthCredential) -> Result<i64> {
    let client = host_metadata_client(metadata_url)?;
    let req = build_request(RequestData::TopUpBandwidth {
        credential: Box::new(credential),
    })?;
    let resp = client
        .topup_bandwidth(&req)
        .await
        .map_err(|e| DvpnError::Transport(format!("topup_bandwidth: {e}")))?;
    match extract_response(&resp)? {
        ResponseData::TopUpBandwidth {
            available_bandwidth,
            ..
        } => Ok(available_bandwidth),
        _ => Err(DvpnError::Transport(
            "unexpected response to top-up request".into(),
        )),
    }
}
