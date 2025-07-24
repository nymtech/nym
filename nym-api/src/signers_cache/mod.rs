// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::signers_cache::cache::refresher::SignersCacheDataProvider;
use crate::signers_cache::cache::SignersCacheData;
use crate::support::caching::cache::SharedCache;
use crate::support::caching::refresher::CacheRefresher;
use crate::support::{config, nyxd};
use nym_task::TaskManager;

pub(crate) mod cache;
pub(crate) mod handlers;

pub(crate) fn start_refresher(
    config: &config::SignersCache,
    nyxd_client: nyxd::Client,
    task_manager: &TaskManager,
) -> SharedCache<SignersCacheData> {
    let refresher = CacheRefresher::new(
        SignersCacheDataProvider::new(nyxd_client),
        config.debug.refresh_interval,
    )
    .named("signers-cache-refresher");
    let shared_cache = refresher.get_shared_cache();
    refresher.start_with_delay(
        task_manager.subscribe_named("signers-cache-refresher"),
        config.debug.refresher_start_delay,
    );
    shared_cache
}
