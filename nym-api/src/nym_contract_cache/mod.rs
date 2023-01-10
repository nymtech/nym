// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use okapi::openapi3::OpenApi;
use rocket::Route;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;
use task::TaskManager;

use crate::epoch_operations::RewardedSetUpdater;
use crate::node_status_api::uptime_updater::HistoricalUptimeUpdater;
use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::config::Config;
use crate::support::{self, nyxd, storage};

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
        routes::get_current_epoch
    ]
}

pub(crate) fn start(
    config: &Config,
    rocket: &rocket::Rocket<rocket::Ignite>,
    nyxd_client: nyxd::Client,
    shutdown: &TaskManager,
) -> Result<tokio::sync::watch::Receiver<support::caching::CacheNotification>, anyhow::Error> {
    let nym_contract_cache_state = rocket
        .state::<NymContractCache>()
        .expect("contract cache has not been setup")
        .clone();

    let nym_contract_cache_refresher = NymContractCacheRefresher::new(
        nyxd_client.clone(),
        config.get_topology_caching_interval(),
        nym_contract_cache_state.clone(),
    );
    let nym_contract_cache_listener = nym_contract_cache_refresher.subscribe();
    let shutdown_listener = shutdown.subscribe();
    tokio::spawn(async move { nym_contract_cache_refresher.run(shutdown_listener).await });

    // TODO: THIS IS VERY MUCH NOT PART OF 'nym_contract_cache'!
    // TODO: THIS IS VERY MUCH NOT PART OF 'nym_contract_cache'!

    // only start the uptime updater if the monitoring if it's enabled
    if config.get_network_monitor_enabled() {
        let storage = rocket
            .state::<storage::NymApiStorage>()
            .expect("api storage has not been setup")
            .clone();

        let uptime_updater = HistoricalUptimeUpdater::new(storage.clone());
        let shutdown_listener = shutdown.subscribe();
        tokio::spawn(async move { uptime_updater.run(shutdown_listener).await });

        // the same idea holds for rewarding
        if config.get_rewarding_enabled() {
            let mut rewarded_set_updater =
                RewardedSetUpdater::new(nyxd_client, nym_contract_cache_state, storage);
            let shutdown_listener = shutdown.subscribe();
            tokio::spawn(async move { rewarded_set_updater.run(shutdown_listener).await.unwrap() });
        }
    }

    Ok(nym_contract_cache_listener)
}
