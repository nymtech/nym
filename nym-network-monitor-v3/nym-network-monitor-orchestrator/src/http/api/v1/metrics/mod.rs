// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::state::AppState;
use crate::orchestrator::prometheus::PROMETHEUS_METRICS;
use axum::Router;
use axum::routing::get;
use nym_network_monitor_orchestrator_requests::routes;

/// Returns `prometheus` compatible metrics
#[utoipa::path(
    get,
    path = "/prometheus",
    context_path = "/v1/metrics",
    tag = "Metrics",
    responses(
        (status = 200, body = String),
        (status = 400, description = "`Authorization` header was missing"),
        (status = 401, description = "Access token is missing or invalid"),
    ),
    security(("metrics_and_results_token" = []))
)]
// the AuthLayer is protecting access to this endpoint
pub(crate) async fn prometheus_metrics() -> String {
    PROMETHEUS_METRICS.metrics()
}

pub(super) fn routes() -> Router<AppState> {
    Router::new().route(routes::v1::metrics::PROMETHEUS, get(prometheus_metrics))
}
