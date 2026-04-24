// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumErrorResponse;
use crate::support::http::state::helpers::ChainSharedCacheWithTtl;
use crate::support::nyxd::Client;
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::nyxd::error::NyxdError;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::RwLock;
use tracing::{error, warn};

/// Per-orchestrator high-water mark of accepted submission timestamps, kept in-memory to provide
/// replay protection for the stress-test submission endpoint.
#[derive(Clone)]
pub(crate) struct LastNMSubmissions {
    pub(crate) submissions: Arc<RwLock<HashMap<ed25519::PublicKey, OffsetDateTime>>>,
}

impl LastNMSubmissions {
    pub(crate) fn new() -> LastNMSubmissions {
        LastNMSubmissions {
            submissions: Arc::new(Default::default()),
        }
    }

    /// Last accepted submission timestamp for particular network monitor
    pub(crate) async fn submitted(&self, nm: ed25519::PublicKey) -> Option<OffsetDateTime> {
        self.submissions.read().await.get(&nm).copied()
    }

    /// Record `timestamp` as the most recent accepted submission for `nm`.
    ///
    /// Callers are responsible for ensuring `timestamp` passes the monotonicity check against
    /// [`submitted`][Self::submitted] before calling this.
    pub(crate) async fn set_submitted(&self, nm: ed25519::PublicKey, timestamp: OffsetDateTime) {
        self.submissions.write().await.insert(nm, timestamp);
    }
}

/// Snapshot of identity keys for network monitor orchestrators currently registered in the
/// network-monitors contract.
#[derive(Clone)]
pub(crate) struct KnownNetworkMonitors {
    known: HashSet<ed25519::PublicKey>,
}

impl KnownNetworkMonitors {
    pub(crate) fn contains(&self, key: &ed25519::PublicKey) -> bool {
        self.known.contains(key)
    }
}

/// TTL-gated cache over [`KnownNetworkMonitors`] so that every submission doesn't re-query the
/// network-monitors contract; refresh happens lazily on the first request after the TTL expires.
#[derive(Clone)]
pub(crate) struct NetworkMonitorsCache(ChainSharedCacheWithTtl<KnownNetworkMonitors>);

impl NetworkMonitorsCache {
    pub(crate) fn new(cache_ttl: Duration) -> Self {
        NetworkMonitorsCache(ChainSharedCacheWithTtl::new(cache_ttl))
    }

    /// Return the currently-cached set of known orchestrators, refreshing from chain if stale.
    pub(crate) async fn get_or_refresh(
        &self,
        client: &Client,
    ) -> Result<KnownNetworkMonitors, AxumErrorResponse> {
        self.0.get_or_refresh(client, refresh).await
    }

    /// Shortcut for "is this key in the current (possibly just-refreshed) orchestrator set?".
    pub(crate) async fn is_authorised(
        &self,
        nyxd_client: &Client,
        key: &ed25519::PublicKey,
    ) -> Result<bool, AxumErrorResponse> {
        Ok(self.get_or_refresh(nyxd_client).await?.known.contains(key))
    }
}

/// Fetch the orchestrator set from the network-monitors contract and decode each entry's identity
/// key. Orchestrators without an announced key, or with an unparseable one, are logged and
/// skipped - the rest still populate the cache so one bad entry doesn't take down submissions for
/// everyone.
async fn refresh(client: &Client) -> Result<KnownNetworkMonitors, NyxdError> {
    if client
        .get_network_monitors_contract_address()
        .await
        .is_err()
    {
        warn!("network monitor contract address not set - can't accept any stress testing results")
    }

    let known_monitors = client.get_all_network_monitor_orchestrators().await?;
    let mut updated_monitors = HashSet::new();
    for monitor in known_monitors {
        let Some(public_key) = monitor.identity_key else {
            warn!("{} orchestrator is authorised but has not announced its public key - is the process running correctly?", monitor.address);
            continue;
        };
        let parsed = match ed25519::PublicKey::from_base58_string(&public_key) {
            Ok(key) => key,
            Err(err) => {
                error!("failed to parse public key for {}: {err}", monitor.address);
                continue;
            }
        };
        updated_monitors.insert(parsed);
    }
    Ok(KnownNetworkMonitors {
        known: updated_monitors,
    })
}
