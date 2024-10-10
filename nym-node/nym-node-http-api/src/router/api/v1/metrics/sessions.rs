// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::state::metrics::MetricsAppState;
use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_requests::api::v1::metrics::models::SessionStats;

/// If applicable, returns sessions statistics information of this node.
/// This information is **PURELY** self-reported and in no way validated.
#[utoipa::path(
    get,
    path = "/sessions",
    context_path = "/api/v1/metrics",
    tag = "Metrics",
    responses(
        (status = 200, content(
            ("application/json" = SessionStats),
            ("application/yaml" = SessionStats)
        ))
    ),
    params(OutputParams),
)]
pub(crate) async fn sessions_stats(
    Query(output): Query<OutputParams>,
    State(metrics_state): State<MetricsAppState>,
) -> SessionStatsResponse {
    let output = output.output.unwrap_or_default();
    let response = metrics_state.session_stats.read().await.as_response();
    output.to_response(response)
}

pub type SessionStatsResponse = FormattedResponse<SessionStats>;
