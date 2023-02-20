// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_task::TaskManager;
use okapi::openapi3::OpenApi;
use rocket::Route;
use rocket_okapi::{openapi_get_routes_spec, settings::OpenApiSettings};

use crate::support::{config::Config, nyxd};

use self::cache::refresher::CirculatingSupplyCacheRefresher;

pub(crate) mod cache;
pub(crate) mod routes;

/// Merges the routes with http information and returns it to Rocket for serving
pub(crate) fn circulating_supply_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        settings: routes::get_full_circulating_supply,
        routes::get_total_supply,
        routes::get_circulating_supply
    ]
}

/// Spawn the circulating supply cache refresher.
pub(crate) fn start_cache_refresh(
    config: &Config,
    nyxd_client: nyxd::Client,
    circulating_supply_cache: &cache::CirculatingSupplyCache,
    shutdown: &TaskManager,
) {
    if config.get_circulating_supply_enabled() {
        let refresher = CirculatingSupplyCacheRefresher::new(
            nyxd_client,
            circulating_supply_cache.to_owned(),
            config.get_circulating_supply_caching_interval(),
        );
        let shutdown_listener = shutdown.subscribe();
        tokio::spawn(async move { refresher.run(shutdown_listener).await });
    }
}
