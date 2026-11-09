// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::cache::refresher::NodeStatusCacheRefresher;
use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_performance::attestation::source::PerformanceSnapshotSource;
use crate::node_performance::provider::legacy_storage_provider::LegacyStoragePerformanceProvider;
use crate::node_status_api::cache::refresher::NodeStatusCacheConfig;
use crate::support::attested_snapshots::state::AttestedSnapshotsState;
use crate::support::caching::cache::SharedCache;
use crate::support::caching::refresher::RefreshRequester;
use crate::support::caching::CacheNotificationWatcher;
use crate::support::config;
use crate::support::nyxd::Client;
use crate::support::storage::NymApiStorage;
pub(crate) use cache::NodeStatusCache;
use cosmwasm_std::Coin;
use nym_task::ShutdownManager;
use std::path::PathBuf;
use std::time::Duration;

pub(crate) mod cache;
pub(crate) mod handlers;
pub(crate) mod helpers;
pub(crate) mod models;
pub(crate) mod uptime_updater;
pub(crate) mod utils;

pub(crate) const FIFTEEN_MINUTES: Duration = Duration::from_secs(900);
pub(crate) const ONE_HOUR: Duration = Duration::from_secs(3600);
pub(crate) const ONE_DAY: Duration = Duration::from_secs(86400);

/// Spawn the node status cache refresher, refreshed on mixnet contract and describe cache updates,
/// with a fallback interval; in contract mode from the performance snapshots.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn start_cache_refresh(
    config: &config::Config,
    chain_client: &Client,
    nym_contract_cache_state: &MixnetContractCache,
    described_cache: &SharedCache<DescribedNodes>,
    performance_snapshots: &AttestedSnapshotsState<PerformanceSnapshotSource>,
    node_status_cache_state: &NodeStatusCache,
    storage: &NymApiStorage,
    nym_contract_cache_listener: CacheNotificationWatcher,
    described_cache_cache_listener: CacheNotificationWatcher,
    on_disk_file: PathBuf,
    shutdown_manager: &ShutdownManager,
) -> RefreshRequester {
    let denom = chain_client.chain_details().await.mix_denom.base;
    let minimum_on_chain_balance = Coin::new(
        config.node_status_api.debug.minimum_on_chain_balance_amount,
        denom,
    );
    let performance_provider = LegacyStoragePerformanceProvider::new(
        storage.clone(),
        nym_contract_cache_state.clone(),
        config.performance_provider.debug.stress_testing_data_period,
        config.performance_provider.debug.liveness_data_period,
    );

    let mut nym_api_cache_refresher = NodeStatusCacheRefresher::new(
        node_status_cache_state.to_owned(),
        NodeStatusCacheConfig::new(config, minimum_on_chain_balance),
        chain_client,
        nym_contract_cache_state.to_owned(),
        described_cache.clone(),
        performance_snapshots.clone(),
        nym_contract_cache_listener,
        described_cache_cache_listener,
        performance_provider,
        on_disk_file,
    );
    let refresh_requester = nym_api_cache_refresher.refresh_requester();
    let shutdown_listener = shutdown_manager.clone_shutdown_token();
    tokio::spawn(async move { nym_api_cache_refresher.run(shutdown_listener).await });
    refresh_requester
}
