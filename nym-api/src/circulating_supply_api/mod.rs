use okapi::openapi3::OpenApi;
use rocket::Route;
use rocket_okapi::{openapi_get_routes_spec, settings::OpenApiSettings};
use task::TaskManager;

use crate::support::{config::Config, nyxd};

use self::cache::refresher::CirculatingSupplyCacheRefresher;

pub(crate) mod cache;
pub(crate) mod routes;

/// Merges the routes with http information and returns it to Rocket for serving
pub(crate) fn circulating_supply_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: routes::get_circulating_supply]
}

/// Spawn the circulating supply cache refresher.
pub(crate) fn start_cache_refresh(
    config: &Config,
    circulating_supply_cache: &cache::CirculatingSupplyCache,
    shutdown: &TaskManager,
) {
    let nyxd_client = nyxd::Client::new_query(config);
    let refresher = CirculatingSupplyCacheRefresher::new(
        nyxd_client,
        circulating_supply_cache.clone(),
        config.get_caching_interval(),
    );
    let shutdown_listener = shutdown.subscribe();
    tokio::spawn(async move { refresher.run(shutdown_listener).await });
}
