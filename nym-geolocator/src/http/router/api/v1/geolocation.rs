// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::state::AppState;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use nym_geolocator_requests::routes::api::v1::geolocation::{
    RECHECK_NODE, RELAY_SELF_DECLARATION, REQUEST_CHECK,
};
use nym_http_api_common::middleware::bearer_auth::AuthLayer;

pub(crate) fn routes(recheck_node_auth: AuthLayer) -> Router<AppState> {
    Router::new()
        .route(REQUEST_CHECK, post(request_geolocation_check))
        .route(RECHECK_NODE, post(recheck_node).layer(recheck_node_auth))
        .route(RELAY_SELF_DECLARATION, post(relay_self_declaration))
}

#[utoipa::path(
    post,
    path = "/request-check",
    context_path = "/api/v1/geolocation",
    tag = "Geolocation",
    responses(
        // TODO
    ),
    request_body(
        // TODO
    ),
)]
async fn request_geolocation_check(
    State(state): State<AppState>,
    Json(body): Json<()>,
) -> axum::response::Response {
    // 1. check if it's even bonded

    // 2. check rate limits

    // 3. perform and submit the check
    todo!()
}

#[utoipa::path(
    post,
    path = "/recheck-node",
    context_path = "/api/v1/geolocation",
    tag = "Geolocation",
    responses(
    // TODO
    ),
    request_body(
    // TODO
    ),
    security(
        ("admin_token" = [])
    )
)]
async fn recheck_node(
    State(state): State<AppState>,
    Json(body): Json<()>,
) -> axum::response::Response {
    todo!()
}

#[utoipa::path(
    post,
    path = "/relay-self-declaration",
    context_path = "/api/v1/geolocation",
    tag = "Geolocation",
    responses(
        // TODO
    ),
    request_body(
        // TODO
    ),
)]
async fn relay_self_declaration(
    State(state): State<AppState>,
    Json(body): Json<()>,
) -> axum::response::Response {
    // 1. check if it's even bonded

    // 2. check if the agent can perform relays

    // 3. verify signature and staleness (to prevent contract rejections)

    // 4. relay
    todo!()
}
