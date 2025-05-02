// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::nym_contract_cache::cache::refresher::ContractDataProvider;
use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::caching::refresher::CacheRefresher;
use crate::support::{self, config, nyxd};
use nym_task::TaskManager;

pub(crate) mod cache;
pub(crate) mod handlers;

pub(crate) fn start_refresher(
    config: &config::NodeStatusAPI,
    nym_contract_cache_state: &NymContractCache,
    nyxd_client: nyxd::Client,
    shutdown: &TaskManager,
) -> tokio::sync::watch::Receiver<support::caching::CacheNotification> {
    CacheRefresher::new_with_initial_value(
        Box::new(ContractDataProvider::new(nyxd_client)),
        config.debug.caching_interval,
        nym_contract_cache_state.inner(),
    )
    .named("contract-cache-refresher")
    .start_with_watcher(shutdown.subscribe())
}
