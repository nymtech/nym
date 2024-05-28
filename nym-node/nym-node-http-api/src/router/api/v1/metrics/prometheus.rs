// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_metrics::metrics;

/// Returns `prometheus` compatible metrics
#[utoipa::path(
    get,
    path = "/prometheus",
    context_path = "/api/v1/metrics",
    tag = "Metrics",
    responses(
        (status = 200, body = String),
        (status = 400, description = "`Authorization` header was missing"),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 500, description = "No access token has been specified on the node"),
    ),
    security(
        ("prometheus_token" = [])
    )
)]
pub(crate) async fn prometheus_metrics() -> String {
    metrics!()
}
