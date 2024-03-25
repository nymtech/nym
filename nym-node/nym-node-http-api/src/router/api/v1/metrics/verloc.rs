// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::state::metrics::MetricsAppState;
use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_requests::api::v1::metrics::models::VerlocStats;

/// If applicable, returns verloc statistics information of this node.
#[utoipa::path(
    get,
    path = "/verloc",
    context_path = "/api/v1/metrics",
    tag = "Metrics",
    responses(
        (status = 200, content(
            ("application/json" = VerlocStats),
            ("application/yaml" = VerlocStats)
        ))
    ),
    params(OutputParams),
)]
pub(crate) async fn verloc_stats(
    Query(output): Query<OutputParams>,
    State(metrics_state): State<MetricsAppState>,
) -> VerlocStatsResponse {
    let output = output.output.unwrap_or_default();
    let response = metrics_state.verloc.read().await.as_response();
    output.to_response(response)
}

pub type VerlocStatsResponse = FormattedResponse<VerlocStats>;
