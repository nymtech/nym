// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::api_routes::aggregation::aggregation_routes;
use crate::ecash::api_routes::issued::issued_routes;
use crate::ecash::api_routes::partial_signing::partial_signing_routes;
use crate::ecash::api_routes::spending::spending_routes;
use crate::support::http::state::AppState;
use axum::Router;

pub(crate) fn ecash_routes() -> Router<AppState> {
    Router::new()
        .merge(aggregation_routes())
        .merge(issued_routes())
        .merge(partial_signing_routes())
        .merge(spending_routes())
}
