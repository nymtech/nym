// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::state::metrics::MetricsAppState;
use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_requests::api::v1::metrics::models::MixingStats;

/// If applicable, returns mixing statistics information of this node.
/// This information is **PURELY** self-reported and in no way validated.
#[utoipa::path(
    get,
    path = "/mixing",
    context_path = "/api/v1/metrics",
    tag = "Metrics",
    responses(
        (status = 200, content(
            ("application/json" = MixingStats),
            ("application/yaml" = MixingStats)
        ))
    ),
    params(OutputParams),
)]
pub(crate) async fn mixing_stats(
    Query(output): Query<OutputParams>,
    State(metrics_state): State<MetricsAppState>,
) -> MixingStatsResponse {
    let output = output.output.unwrap_or_default();
    let response = metrics_state.mixing_stats.read().await.as_response();
    output.to_response(response)
}

pub type MixingStatsResponse = FormattedResponse<MixingStats>;
