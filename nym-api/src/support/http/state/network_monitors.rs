// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumErrorResponse;
use crate::support::http::state::helpers::ChainSharedCacheWithTtl;
use crate::support::nyxd::Client;
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::nyxd::error::NyxdError;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::warn;

#[derive(Clone)]
pub(crate) struct KnownNetworkMonitors {
    monitors: Arc<RwLock<HashSet<ed25519::PublicKey>>>,
}

impl KnownNetworkMonitors {
    pub(crate) async fn contains(&self, key: &ed25519::PublicKey) -> bool {
        self.monitors.read().await.contains(key)
    }
}

#[derive(Clone)]
pub(crate) struct NetworkMonitorsCache(ChainSharedCacheWithTtl<KnownNetworkMonitors>);

impl NetworkMonitorsCache {
    pub(crate) fn new(cache_ttl: Duration) -> Self {
        NetworkMonitorsCache(ChainSharedCacheWithTtl::new(cache_ttl))
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
    todo!()
}

impl NetworkMonitorsCache {
    pub(crate) async fn get_or_refresh(
        &self,
        client: &Client,
    ) -> Result<KnownNetworkMonitors, AxumErrorResponse> {
        self.0.get_or_refresh(client, refresh).await
    }
}
