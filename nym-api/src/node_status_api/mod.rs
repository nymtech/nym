// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::cache::refresher::NodeStatusCacheRefresher;
use crate::support::config;
use crate::{
    nym_contract_cache::cache::NymContractCache,
    support::{self, storage},
};
pub(crate) use cache::NodeStatusCache;
use nym_task::TaskManager;
use std::time::Duration;

pub(crate) mod cache;
pub(crate) mod handlers;
pub(crate) mod helpers;
pub(crate) mod models;
pub(crate) mod reward_estimate;
pub(crate) mod uptime_updater;
pub(crate) mod utils;

pub(crate) const FIFTEEN_MINUTES: Duration = Duration::from_secs(900);
pub(crate) const ONE_HOUR: Duration = Duration::from_secs(3600);
pub(crate) const ONE_DAY: Duration = Duration::from_secs(86400);

/// Spawn the node status cache refresher.
///
/// It is primarily refreshed in-sync with the nym contract cache, however provide a fallback
/// caching interval that is twice the nym contract cache
pub(crate) fn start_cache_refresh(
    config: &config::NodeStatusAPI,
    nym_contract_cache_state: &NymContractCache,
    node_status_cache_state: &NodeStatusCache,
    storage: storage::NymApiStorage,
    nym_contract_cache_listener: tokio::sync::watch::Receiver<support::caching::CacheNotification>,
    shutdown: &TaskManager,
) {
    let mut nym_api_cache_refresher = NodeStatusCacheRefresher::new(
        node_status_cache_state.to_owned(),
        config.debug.caching_interval,
        nym_contract_cache_state.to_owned(),
        nym_contract_cache_listener,
        storage,
    );
    let shutdown_listener = shutdown.subscribe();
    tokio::spawn(async move { nym_api_cache_refresher.run(shutdown_listener).await });
}
