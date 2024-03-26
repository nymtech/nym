// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::state::metrics::MetricsAppState;
use axum::extract::State;
use axum::headers::authorization::Bearer;
use axum::headers::Authorization;
use axum::http::StatusCode;
use axum::TypedHeader;
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
pub(crate) async fn prometheus_metrics<'a>(
    TypedHeader(authorization): TypedHeader<Authorization<Bearer>>,
    State(state): State<MetricsAppState>,
) -> Result<String, StatusCode> {
    if authorization.token().is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    // TODO: is 500 the correct error code here?
    let Some(metrics_key) = state.prometheus_access_token else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    if metrics_key != authorization.token() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(metrics!())
}
