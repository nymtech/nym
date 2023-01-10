// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use okapi::openapi3::OpenApi;
use rocket::Route;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;
use task::TaskManager;

use crate::epoch_operations::RewardedSetUpdater;
use crate::node_status_api::uptime_updater::HistoricalUptimeUpdater;
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

/// Starts a cache refresher for nym contract data, with a signing nyxd client.
///
/// DH question: why do we need to pass in the signing nyxd client? It seems like we could just
/// use the one that's already in the rocket state.
pub(crate) async fn start_with_signing(
    rocket: &rocket::Rocket<rocket::Ignite>,
    shutdown: &TaskManager,
    signing_nyxd_client: &nyxd::Client<validator_client::nyxd::SigningNyxdClient>,
    config: &Config,
    nym_contract_cache: &cache::NymContractCache,
) -> Result<tokio::sync::watch::Receiver<support::caching::CacheNotification>, anyhow::Error> {
    let storage = rocket.state::<storage::NymApiStorage>().unwrap().clone();
    let uptime_updater = HistoricalUptimeUpdater::new(storage.clone());
    let shutdown_listener = shutdown.subscribe();
    tokio::spawn(async move { uptime_updater.run(shutdown_listener).await });

    let nym_contract_cache_refresher = NymContractCacheRefresher::new(
        signing_nyxd_client.clone(),
        config.get_caching_interval(),
        nym_contract_cache.clone(),
    );
    let nym_contract_cache_listener = nym_contract_cache_refresher.subscribe();
    let shutdown_listener = shutdown.subscribe();
    tokio::spawn(async move { nym_contract_cache_refresher.run(shutdown_listener).await });

    if config.get_rewarding_enabled() {
        let mut rewarded_set_updater = RewardedSetUpdater::new(
            signing_nyxd_client.clone(),
            nym_contract_cache.clone(),
            storage,
        )
        .await?;
        let shutdown_listener = shutdown.subscribe();
        tokio::spawn(async move { rewarded_set_updater.run(shutdown_listener).await.unwrap() });
    }

    Ok(nym_contract_cache_listener)
}

/// Spawn the nym contract cache refresher.
/// When the network monitor is not enabled, we spawn the nym contract cache refresher task
/// with just a nyxd query client, as there's no need for a nyxd signing client.
pub(crate) fn start_without_signing(
    config: &Config,
    nym_contract_cache: &cache::NymContractCache,
    shutdown: &TaskManager,
) -> tokio::sync::watch::Receiver<support::caching::CacheNotification> {
    let nyxd_client = nyxd::Client::new_query(config);
    let nym_contract_cache_refresher = NymContractCacheRefresher::new(
        nyxd_client,
        config.get_caching_interval(),
        nym_contract_cache.clone(),
    );
    let nym_contract_cache_listener = nym_contract_cache_refresher.subscribe();
    let shutdown_listener = shutdown.subscribe();
    tokio::spawn(async move { nym_contract_cache_refresher.run(shutdown_listener).await });
    nym_contract_cache_listener
}
