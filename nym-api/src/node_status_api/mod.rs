// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::cache::refresher::NodeStatusCacheRefresher;
use crate::support::config;
use crate::{
    nym_contract_cache::cache::NymContractCache,
    support::{self, storage},
};
pub(crate) use cache::NodeStatusCache;
use nym_task::TaskManager;
use okapi::openapi3::OpenApi;
use rocket::Route;
use rocket_okapi::{openapi_get_routes_spec, settings::OpenApiSettings};
use std::time::Duration;

pub(crate) mod cache;
#[cfg(feature = "axum")]
pub(crate) mod handlers;
#[cfg(feature = "axum")]
pub(crate) mod helpers;
pub(crate) mod helpers_deprecated;
pub(crate) mod models;
pub(crate) mod reward_estimate;
pub(crate) mod routes_deprecated;
pub(crate) mod uptime_updater;
pub(crate) mod utils;

pub(crate) const FIFTEEN_MINUTES: Duration = Duration::from_secs(900);
pub(crate) const ONE_HOUR: Duration = Duration::from_secs(3600);
pub(crate) const ONE_DAY: Duration = Duration::from_secs(86400);

pub(crate) fn node_status_routes(
    settings: &OpenApiSettings,
    enabled: bool,
) -> (Vec<Route>, OpenApi) {
    if enabled {
        openapi_get_routes_spec![
            settings: routes_deprecated::gateway_report,
            routes_deprecated::gateway_uptime_history,
            routes_deprecated::gateway_core_status_count,
            routes_deprecated::mixnode_report,
            routes_deprecated::mixnode_uptime_history,
            routes_deprecated::mixnode_core_status_count,
            routes_deprecated::get_mixnode_status,
            routes_deprecated::get_mixnode_reward_estimation,
            routes_deprecated::compute_mixnode_reward_estimation,
            routes_deprecated::get_mixnode_stake_saturation,
            routes_deprecated::get_mixnode_inclusion_probability,
            routes_deprecated::get_mixnode_avg_uptime,
            routes_deprecated::get_gateway_avg_uptime,
            routes_deprecated::get_mixnode_inclusion_probabilities,
            routes_deprecated::get_mixnodes_detailed,
            routes_deprecated::get_mixnodes_detailed_unfiltered,
            routes_deprecated::get_rewarded_set_detailed,
            routes_deprecated::get_active_set_detailed,
            routes_deprecated::get_gateways_detailed,
            routes_deprecated::get_gateways_detailed_unfiltered,
            routes_deprecated::unstable::mixnode_test_results,
            routes_deprecated::unstable::gateway_test_results,
            routes_deprecated::submit_gateway_monitoring_results,
            routes_deprecated::submit_node_monitoring_results,
        ]
    } else {
        // in the minimal variant we would not have access to endpoints relying on existence
        // of the network monitor and the associated storage
        openapi_get_routes_spec![
            settings: routes_deprecated::get_mixnode_status,
            routes_deprecated::get_mixnode_stake_saturation,
            routes_deprecated::get_mixnode_inclusion_probability,
            routes_deprecated::get_mixnode_inclusion_probabilities,
            routes_deprecated::get_mixnodes_detailed,
            routes_deprecated::get_rewarded_set_detailed,
            routes_deprecated::get_active_set_detailed,
        ]
    }
}

/// Spawn the node status cache refresher.
///
/// It is primarily refreshed in-sync with the nym contract cache, however provide a fallback
/// caching interval that is twice the nym contract cache
pub(crate) fn start_cache_refresh(
    config: &config::NodeStatusAPI,
    nym_contract_cache_state: &NymContractCache,
    node_status_cache_state: &NodeStatusCache,
    storage: storage::NymApiStorage,
    nym_contract_cache_listener: tokio::sync::watch::Receiver<support::caching::CacheNotification>,
    shutdown: &TaskManager,
) {
    let mut nym_api_cache_refresher = NodeStatusCacheRefresher::new(
        node_status_cache_state.to_owned(),
        config.debug.caching_interval,
        nym_contract_cache_state.to_owned(),
        nym_contract_cache_listener,
        storage,
    );
    let shutdown_listener = shutdown.subscribe();
    tokio::spawn(async move { nym_api_cache_refresher.run(shutdown_listener).await });
}
