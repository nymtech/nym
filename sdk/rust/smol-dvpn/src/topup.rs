// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Background bandwidth top-up (OpenSpec task 4.9).
//!
//! A long-lived tunnel's registered bandwidth is finite. This module polls the
//! gateway `metadata` endpoint's `available_bandwidth` and, when it falls below
//! a threshold, spends one stored ticket via `topup_bandwidth` to extend it.
//!
//! The datapath stays decoupled from provisioning: the caller supplies a
//! [`BandwidthCredentialSource`] (typically backed by
//! `nym-sdk-session::Session::obtain_wireguard_credential`) rather than this
//! crate depending on the credential store or chain client.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use nym_credentials_interface::BandwidthCredential;
use nym_http_api_client::Client as MetadataHttpClient;
use nym_wireguard_private_metadata_client::WireguardMetadataApiClient;
use nym_wireguard_private_metadata_shared::interface::{RequestData, ResponseData};
use nym_wireguard_private_metadata_shared::{Construct, Extract, Request, Version};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::error::{DvpnError, Result};

/// Metadata protocol version this client speaks.
const PROTOCOL_VERSION: Version = Version::V2;
/// Default low-water mark: top up when under 100 MiB remains.
const DEFAULT_THRESHOLD_BYTES: i64 = 100 * 1024 * 1024;
/// Default poll cadence.
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(60);

/// Future returned by a [`BandwidthCredentialSource`].
pub type CredentialFuture<'a> =
    Pin<Box<dyn Future<Output = Result<BandwidthCredential>> + Send + 'a>>;

/// Supplies bandwidth credentials on demand — one spent ticket per call.
pub trait BandwidthCredentialSource: Send + Sync {
    /// Obtain a fresh spendable credential (spends one stored ticket).
    fn obtain(&self) -> CredentialFuture<'_>;
}

/// Configuration for the background top-up task.
#[derive(Clone, Debug)]
pub struct TopupConfig {
    /// Base URL of the gateway `metadata` HTTP endpoint.
    pub metadata_url: String,
    /// Top up when available bandwidth drops below this many bytes.
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

fn extract_response(
    resp: &nym_wireguard_private_metadata_shared::Response,
) -> Result<ResponseData> {
    <_ as Extract<ResponseData>>::extract(resp)
        .map(|(data, _version)| data)
        .map_err(|e| DvpnError::Transport(format!("decode metadata response: {e}")))
}

/// Query the gateway for currently available bandwidth (bytes).
pub async fn available_bandwidth(client: &MetadataHttpClient) -> Result<i64> {
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

/// Spend one credential against the gateway `metadata` endpoint, returning the
/// updated available bandwidth (bytes).
pub async fn topup_once(
    client: &MetadataHttpClient,
    credential: BandwidthCredential,
) -> Result<i64> {
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

/// Build a metadata HTTP client for the given endpoint URL.
pub(crate) fn metadata_client(url: &str) -> Result<MetadataHttpClient> {
    MetadataHttpClient::new_url(url, None)
        .map_err(|e| DvpnError::Config(format!("invalid metadata url {url}: {e}")))
}

/// One-shot: query available bandwidth (bytes) at a gateway metadata endpoint.
pub async fn query_available_bandwidth(metadata_url: &str) -> Result<i64> {
    available_bandwidth(&metadata_client(metadata_url)?).await
}

/// One-shot: spend `credential` at a gateway metadata endpoint, returning the
/// updated available bandwidth (bytes).
pub async fn topup_bandwidth(metadata_url: &str, credential: BandwidthCredential) -> Result<i64> {
    topup_once(&metadata_client(metadata_url)?, credential).await
}

/// The background top-up task: poll available bandwidth and top up before it is
/// exhausted. Runs until the `CancellationToken` fires.
pub(crate) async fn run_topup(
    config: TopupConfig,
    source: Arc<dyn BandwidthCredentialSource>,
    cancel: CancellationToken,
) {
    let client = match metadata_client(&config.metadata_url) {
        Ok(c) => c,
        Err(e) => {
            warn!("bandwidth top-up disabled: {e}");
            return;
        }
    };

    let mut ticker = tokio::time::interval(config.poll_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                debug!("bandwidth top-up task cancelled");
                break;
            }
            _ = ticker.tick() => {
                match available_bandwidth(&client).await {
                    Ok(avail) if avail < config.threshold_bytes => {
                        info!(available = avail, threshold = config.threshold_bytes,
                              "bandwidth low; topping up");
                        match source.obtain().await {
                            Ok(cred) => match topup_once(&client, cred).await {
                                Ok(new_avail) => info!(available = new_avail, "topped up"),
                                Err(e) => warn!("top-up failed: {e}"),
                            },
                            Err(e) => warn!("could not obtain credential for top-up: {e}"),
                        }
                    }
                    Ok(_) => {}
                    Err(e) => debug!("available-bandwidth query failed: {e}"),
                }
            }
        }
    }
}
