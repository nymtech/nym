// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use okapi::openapi3::OpenApi;
use rocket::Route;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;

pub(crate) mod routes;

/// Merges the routes with http information and returns it to Rocket for serving
pub(crate) fn nym_node_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        settings: routes::get_gateways_described
    ]
}
