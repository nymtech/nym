use okapi::openapi3::OpenApi;
use rocket::Route;
use rocket_okapi::{openapi_get_routes_spec, settings::OpenApiSettings};
use validator_client::nymd::CosmWasmClient;

pub(crate) mod cache;
pub(crate) mod routes;

// Merges the routes with openapi information and returns it to Rocket for serving
pub(crate) fn circulating_supply_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: routes::get_circulating_supply]
}
