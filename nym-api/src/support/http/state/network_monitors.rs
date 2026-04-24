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

    pub(crate) async fn submitted(&self, nm: ed25519::PublicKey) -> OffsetDateTime {
        // if entry does not exist (e.g. we have restarted),
        // we play it safe and use the current timestamp
        self.submissions
            .read()
            .await
            .get(&nm)
            .copied()
            .unwrap_or_else(|| OffsetDateTime::now_utc())
    }

    pub(crate) async fn set_submitted(&self, nm: ed25519::PublicKey, timestamp: OffsetDateTime) {
        self.submissions.write().await.insert(nm, timestamp);
    }
}

#[derive(Clone)]
pub(crate) struct KnownNetworkMonitors {
    known: HashSet<ed25519::PublicKey>,
}

impl KnownNetworkMonitors {
    pub(crate) fn contains(&self, key: &ed25519::PublicKey) -> bool {
        self.known.contains(key)
    }
}

#[derive(Clone)]
pub(crate) struct NetworkMonitorsCache(ChainSharedCacheWithTtl<KnownNetworkMonitors>);

impl NetworkMonitorsCache {
    pub(crate) fn new(cache_ttl: Duration) -> Self {
        NetworkMonitorsCache(ChainSharedCacheWithTtl::new(cache_ttl))
    }

    pub(crate) async fn get_or_refresh(
        &self,
        client: &Client,
    ) -> Result<KnownNetworkMonitors, AxumErrorResponse> {
        self.0.get_or_refresh(client, refresh).await
    }

    pub(crate) async fn is_authorised(
        &self,
        nyxd_client: &Client,
        key: &ed25519::PublicKey,
    ) -> Result<bool, AxumErrorResponse> {
        Ok(self.get_or_refresh(nyxd_client).await?.known.contains(key))
    }
}

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
