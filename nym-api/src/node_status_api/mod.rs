// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::cache::refresher::NodeStatusCacheRefresher;
use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_performance::provider::NodePerformanceProvider;
use crate::node_status_api::cache::refresher::NodeStatusCacheConfig;
use crate::support::caching::cache::SharedCache;
use crate::support::caching::refresher::RefreshRequester;
use crate::support::config;
use crate::{
    mixnet_contract_cache::cache::MixnetContractCache,
    support::{self},
};
pub(crate) use cache::NodeStatusCache;
use nym_task::ShutdownManager;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::watch;

pub(crate) mod cache;
pub(crate) mod handlers;
pub(crate) mod helpers;
pub(crate) mod models;
pub(crate) mod uptime_updater;
pub(crate) mod utils;

pub(crate) const FIFTEEN_MINUTES: Duration = Duration::from_secs(900);
pub(crate) const ONE_HOUR: Duration = Duration::from_secs(3600);
pub(crate) const ONE_DAY: Duration = Duration::from_secs(86400);

/// Spawn the node status cache refresher.
///
/// It is primarily refreshed in-sync with the contract cache and described, however provide a fallback
/// caching interval that is twice the nym contract cache
#[allow(clippy::too_many_arguments)]
pub(crate) fn start_cache_refresh(
    config: &config::Config,
    nym_contract_cache_state: &MixnetContractCache,
    described_cache: &SharedCache<DescribedNodes>,
    node_status_cache_state: &NodeStatusCache,
    performance_provider: Box<dyn NodePerformanceProvider + Send + Sync>,
    nym_contract_cache_listener: watch::Receiver<support::caching::CacheNotification>,
    described_cache_cache_listener: watch::Receiver<support::caching::CacheNotification>,
    on_disk_file: PathBuf,
    shutdown_manager: &ShutdownManager,
) -> RefreshRequester {
    let config = NodeStatusCacheConfig {
        fallback_caching_interval: config.node_status_api.debug.caching_interval,
        use_stress_testing_data: config.performance_provider.debug.use_stress_testing_data,
        minimum_available_stress_testing_results: config
            .performance_provider
            .debug
            .minimum_available_stress_testing_results,
        stress_testing_score_weight: config
            .performance_provider
            .debug
            .stress_testing_score_weight,
    };

    let mut nym_api_cache_refresher = NodeStatusCacheRefresher::new(
        node_status_cache_state.to_owned(),
        config,
        nym_contract_cache_state.to_owned(),
        described_cache.clone(),
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
