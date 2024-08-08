// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::api_routes::aggregation_axum::aggregation_routes;
use crate::ecash::api_routes::issued_axum::issued_routes;
use crate::ecash::api_routes::partial_signing_axum::partial_signing_routes;
use crate::ecash::api_routes::spending_axum::spending_routes;
use crate::ecash::state::EcashState;
use crate::v2::AxumAppState;
use axum::Router;
use std::sync::Arc;

pub(crate) fn ecash_routes(ecash_state: Arc<EcashState>) -> Router<AxumAppState> {
    Router::new().nest(
        "/v1",
        Router::new()
            .merge(aggregation_routes(Arc::clone(&ecash_state)))
            .merge(issued_routes(Arc::clone(&ecash_state)))
            .merge(partial_signing_routes(Arc::clone(&ecash_state)))
            .merge(spending_routes(Arc::clone(&ecash_state))),
    )
}
