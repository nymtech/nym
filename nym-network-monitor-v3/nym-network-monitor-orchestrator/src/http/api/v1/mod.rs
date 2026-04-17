// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::state::AppState;
use axum::Router;
use nym_http_api_common::middleware::bearer_auth::AuthLayer;
use nym_network_monitor_orchestrator_requests::routes;

pub(crate) mod agent;
pub(crate) mod error;
pub(crate) mod metrics;
pub(crate) mod results;

/// Assembles the v1 API router, nesting agent, metrics, and results sub-routers
/// under their respective path prefixes. Metrics and results share the same
/// bearer-auth layer.
pub(crate) fn routes(
    agents_auth: AuthLayer,
    metrics_and_results_auth: AuthLayer,
) -> Router<AppState> {
    let metrics_and_results = Router::new()
        .nest(routes::v1::METRICS, metrics::routes())
        .nest(routes::v1::RESULTS, results::routes())
        .route_layer(metrics_and_results_auth);

    Router::new()
        .nest(routes::v1::AGENT, agent::routes(agents_auth))
        .merge(metrics_and_results)
}
