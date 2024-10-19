// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::{self, config, nyxd};
use nym_task::TaskManager;

use self::cache::refresher::NymContractCacheRefresher;

pub(crate) mod cache;
pub(crate) mod handlers;

pub(crate) fn start_refresher(
    config: &config::NodeStatusAPI,
    nym_contract_cache_state: &NymContractCache,
    nyxd_client: nyxd::Client,
    shutdown: &TaskManager,
) -> tokio::sync::watch::Receiver<support::caching::CacheNotification> {
    let nym_contract_cache_refresher = NymContractCacheRefresher::new(
        nyxd_client,
        config.debug.caching_interval,
        nym_contract_cache_state.to_owned(),
    );
    let nym_contract_cache_listener = nym_contract_cache_refresher.subscribe();
    let shutdown_listener = shutdown.subscribe();
    tokio::spawn(async move { nym_contract_cache_refresher.run(shutdown_listener).await });

    nym_contract_cache_listener
}
