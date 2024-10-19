// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::api_routes::aggregation::aggregation_routes;
use crate::ecash::api_routes::issued::issued_routes;
use crate::ecash::api_routes::partial_signing::partial_signing_routes;
use crate::ecash::api_routes::spending::spending_routes;
use crate::ecash::state::EcashState;
use crate::support::http::state::AppState;
use axum::Router;
use std::sync::Arc;

pub(crate) fn ecash_routes(ecash_state: Arc<EcashState>) -> Router<AppState> {
    Router::new()
        .merge(aggregation_routes(Arc::clone(&ecash_state)))
        .merge(issued_routes(Arc::clone(&ecash_state)))
        .merge(partial_signing_routes(Arc::clone(&ecash_state)))
        .merge(spending_routes(Arc::clone(&ecash_state)))
}
