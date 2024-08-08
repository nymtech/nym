// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::http;
use crate::v2::AxumAppState;
use axum::Router;
use okapi::openapi3::OpenApi;
use rocket::Route;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;

pub(crate) mod handlers;
pub(crate) mod handlers_unstable;
pub(crate) mod routes;
mod unstable_routes;

pub(crate) fn nym_node_routes() -> axum::Router<AxumAppState> {
    Router::new().route(
        &http::static_routes::v1::gateways::described(),
        axum::routing::get(handlers::get_gateways_described),
    )
}

pub(crate) fn nym_node_routes_unstable() -> axum::Router<AxumAppState> {
    Router::new().nest(
        "/v1/unstable/nym-nodes",
        Router::new()
            .route(
                "/skimmed",
                axum::routing::get(handlers_unstable::nodes_basic),
            )
            .route(
                "/semi-skimmed",
                axum::routing::get(handlers_unstable::nodes_expanded),
            )
            .route(
                "/full-fat",
                axum::routing::get(handlers_unstable::nodes_detailed),
            )
            .nest(
                "/gateways",
                Router::new()
                    .route(
                        "/skimmed",
                        axum::routing::get(handlers_unstable::gateways_basic),
                    )
                    .route(
                        "/semi-skimmed",
                        axum::routing::get(handlers_unstable::gateways_expanded),
                    )
                    .route(
                        "/full-fat",
                        axum::routing::get(handlers_unstable::gateways_detailed),
                    ),
            )
            .nest(
                "/mixnodes",
                Router::new()
                    .route(
                        "/skimmed",
                        axum::routing::get(handlers_unstable::mixnodes_basic),
                    )
                    .route(
                        "/semi-skimmed",
                        axum::routing::get(handlers_unstable::mixnodes_expanded),
                    )
                    .route(
                        "/full-fat",
                        axum::routing::get(handlers_unstable::mixnodes_detailed),
                    ),
            ),
    )
}

/// Merges the routes with http information and returns it to Rocket for serving
pub(crate) fn nym_node_routes_deprecated(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        settings: routes::get_gateways_described, routes::get_mixnodes_described
    ]
}

pub(crate) fn nym_node_routes_next(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        settings:
        unstable_routes::nodes_basic,
        unstable_routes::nodes_expanded,
        unstable_routes::nodes_detailed,
        unstable_routes::gateways_basic,
        unstable_routes::gateways_expanded,
        unstable_routes::gateways_detailed,
        unstable_routes::mixnodes_basic,
        unstable_routes::mixnodes_expanded,
        unstable_routes::mixnodes_detailed,
    ]
}
