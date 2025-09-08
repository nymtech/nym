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
use anyhow::bail;
use nym_task::ShutdownManager;

pub(crate) mod data;
pub(crate) mod refresher;

pub(crate) async fn start_cache_refresher(
    config: &config::PerformanceProvider,
    nyxd_client: nyxd::Client,
    mixnet_contract_cache: MixnetContractCache,
    shutdown_manager: &ShutdownManager,
) -> anyhow::Result<SharedCache<PerformanceContractCacheData>> {
    let values_to_retain = config.debug.max_epoch_entries_to_retain;

    let mut item_provider =
        PerformanceContractDataProvider::new(nyxd_client, mixnet_contract_cache);

    if !item_provider.cache_has_values().await {
        bail!("performance contract is empty - can't use it as source of node performance")
    }

    let warmed_up_cache = SharedCache::new_with_value(
        item_provider
            .provide_initial_warmed_up_cache(values_to_retain)
            .await?,
    );

    CacheRefresher::new_with_initial_value(
        Box::new(item_provider),
        config.debug.contract_polling_interval,
        warmed_up_cache.clone(),
    )
    .named("performance-contract-cache-refresher")
    .with_update_fn(move |main_cache, update| {
        refresher_update_fn(main_cache, update, values_to_retain)
    })
    .start(shutdown_manager.clone_token("performance-contract-cache-refresher"));

    Ok(warmed_up_cache)
}
