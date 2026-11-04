// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::cache::refresher::NodeStatusCacheRefresher;
use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::support::attested_snapshots::state::ContractSnapshots;
use crate::support::caching::refresher::RefreshRequester;
use crate::support::caching::CacheNotificationWatcher;
pub(crate) use cache::NodeStatusCache;
use nym_task::ShutdownManager;
use std::path::PathBuf;
use std::time::Duration;

pub(crate) mod cache;
pub(crate) mod handlers;
pub(crate) mod helpers;
pub(crate) mod models;

pub(crate) const ONE_DAY: Duration = Duration::from_secs(86400);

/// Spawn the node status cache refresher, building the annotations from the performance snapshots,
/// refreshed on mixnet contract cache updates, with a fallback interval.
pub(crate) fn start_cache_refresh(
    fallback_caching_interval: Duration,
    nym_contract_cache_state: &MixnetContractCache,
    contract_snapshots: &ContractSnapshots,
    node_status_cache_state: &NodeStatusCache,
    nym_contract_cache_listener: CacheNotificationWatcher,
    on_disk_file: PathBuf,
    shutdown_manager: &ShutdownManager,
) -> RefreshRequester {
    let mut nym_api_cache_refresher = NodeStatusCacheRefresher::new(
        node_status_cache_state.to_owned(),
        fallback_caching_interval,
        nym_contract_cache_state.to_owned(),
        contract_snapshots.clone(),
        nym_contract_cache_listener,
        on_disk_file,
    );
    let refresh_requester = nym_api_cache_refresher.refresh_requester();
    let shutdown_listener = shutdown_manager.clone_shutdown_token();
    tokio::spawn(async move { nym_api_cache_refresher.run(shutdown_listener).await });
    refresh_requester
}
