// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::{self, config, nyxd};
use nym_task::TaskManager;
use okapi::openapi3::OpenApi;
use rocket::Route;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;

use self::cache::refresher::NymContractCacheRefresher;

pub(crate) mod cache;
pub(crate) mod routes;

pub(crate) fn nym_contract_cache_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        settings: routes::get_mixnodes,
        routes::get_mixnodes_detailed,
        routes::get_gateways,
        routes::get_active_set,
        routes::get_active_set_detailed,
        routes::get_rewarded_set,
        routes::get_rewarded_set_detailed,
        routes::get_blacklisted_mixnodes,
        routes::get_blacklisted_gateways,
        routes::get_interval_reward_params,
        routes::get_current_epoch,
        routes::get_services,
    ]
}

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
