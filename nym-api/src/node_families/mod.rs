// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::node_families::cache::refresher::NodeFamiliesDataProvider;
use crate::node_families::cache::NodeFamiliesCacheData;
use crate::support::caching::cache::SharedCache;
use crate::support::caching::refresher::CacheRefresher;
use crate::support::{config, nyxd};
use nym_validator_client::nyxd::error::NyxdError;
use std::path::PathBuf;

pub(crate) mod cache;
pub(crate) mod handlers;
#[cfg(test)]
mod tests;

pub(crate) fn build_refresher(
    config: &config::NodeFamiliesCache,
    mixnet_contract_cache: &MixnetContractCache,
    node_families_cache: &SharedCache<NodeFamiliesCacheData>,
    nyxd_client: nyxd::Client,
    on_disk_file: PathBuf,
) -> CacheRefresher<NodeFamiliesCacheData, NyxdError> {
    CacheRefresher::new_with_initial_value(
        Box::new(NodeFamiliesDataProvider::new(
            config.debug.node_families_block_timestamp_fetch_concurrency,
            nyxd_client,
            mixnet_contract_cache.clone(),
            node_families_cache.clone(),
        )),
        config.debug.caching_interval,
        node_families_cache.clone(),
    )
    .named("node-families-cache-refresher")
    .with_persistent_cache(on_disk_file)
}
