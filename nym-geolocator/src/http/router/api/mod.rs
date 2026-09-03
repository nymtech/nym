// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::state::AppState;
use axum::Router;
use nym_geolocator_requests::routes::api::V1;
use nym_http_api_common::middleware::bearer_auth::AuthLayer;

pub(crate) mod openapi;
pub(crate) mod v1;

pub(crate) fn routes(recheck_node_auth: AuthLayer) -> Router<AppState> {
    Router::new().nest(V1, v1::v1_routes(recheck_node_auth))
}
