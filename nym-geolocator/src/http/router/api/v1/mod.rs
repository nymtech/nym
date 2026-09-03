// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::state::AppState;
use axum::Router;
use nym_geolocator_requests::routes::api::v1::GEOLOCATION;
use nym_http_api_common::middleware::bearer_auth::AuthLayer;

pub(crate) mod geolocation;

pub(crate) fn v1_routes(recheck_node_auth: AuthLayer) -> Router<AppState> {
    Router::new().nest(GEOLOCATION, geolocation::routes(recheck_node_auth))
}
