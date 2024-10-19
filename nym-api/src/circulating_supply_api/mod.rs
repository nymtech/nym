// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::cache::refresher::CirculatingSupplyCacheRefresher;
use crate::support::{config, nyxd};
use nym_task::TaskManager;

pub(crate) mod cache;
pub(crate) mod handlers;

/// Spawn the circulating supply cache refresher.
pub(crate) fn start_cache_refresh(
    config: &config::CirculatingSupplyCacher,
    nyxd_client: nyxd::Client,
    circulating_supply_cache: &cache::CirculatingSupplyCache,
    shutdown: &TaskManager,
) {
    if config.enabled {
        let refresher = CirculatingSupplyCacheRefresher::new(
            nyxd_client,
            circulating_supply_cache.to_owned(),
            config.debug.caching_interval,
        );
        let shutdown_listener = shutdown.subscribe();
        tokio::spawn(async move { refresher.run(shutdown_listener).await });
    }
}
