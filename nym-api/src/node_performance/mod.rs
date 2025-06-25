// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::node_performance::contract_cache::data::PerformanceContractCacheData;
use crate::node_performance::contract_cache::refresher::{
    refresher_update_fn, PerformanceContractDataProvider,
};
use crate::support::caching::cache::SharedCache;
use crate::support::caching::refresher::CacheRefresher;
use crate::support::{config, nyxd};
use nym_task::TaskManager;

pub(crate) mod contract_cache;
pub(crate) mod legacy_storage_provider;
pub(crate) mod provider;

pub(crate) fn start_cache_refresher(
    config: &config::PerformanceProvider,
    nyxd_client: nyxd::Client,
    mixnet_contract_cache: MixnetContractCache,
    task_manager: &TaskManager,
) -> SharedCache<PerformanceContractCacheData> {
    let values_to_retain = config.debug.max_epoch_entries_to_retain;

    // if we crash just before the legacy data has to be updated...
    let todo = "actually we need to warm up our cache with the last retention amount of values";

    let item_provider = PerformanceContractDataProvider::new(nyxd_client, mixnet_contract_cache);
    let refresher = CacheRefresher::new(item_provider, config.debug.contract_polling_interval)
        .named("performance-contract-cache-refresher")
        .with_update_fn(move |main_cache, update| {
            refresher_update_fn(main_cache, update, values_to_retain)
        });

    let shared_cache = refresher.get_shared_cache();

    refresher.start(task_manager.subscribe_named("performance-contract-cache-refresher"));
    shared_cache
}
