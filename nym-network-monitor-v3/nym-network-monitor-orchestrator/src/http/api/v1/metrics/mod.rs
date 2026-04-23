// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::state::AppState;
use axum::Router;
use nym_http_api_common::middleware::bearer_auth::AuthLayer;

pub(super) fn routes(auth_layer: AuthLayer) -> Router<AppState> {
    let _ = auth_layer;
    Router::new()
}
