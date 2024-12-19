// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_requests::api::v1::metrics::models::{
    VerlocNodeResult, VerlocResult, VerlocResultData, VerlocStats,
};
use nym_verloc::measurements::SharedVerlocStats;

use crate::node::http::state::metrics::MetricsAppState;

/// If applicable, returns verloc statistics information of this node.
#[utoipa::path(
    get,
    path = "/verloc",
    context_path = "/api/v1/metrics",
    tag = "Metrics",
    responses(
        (status = 200, content(
            (VerlocStats = "application/json"),
            (VerlocStats = "application/yaml")
        ))
    ),
    params(OutputParams),
)]
pub(crate) async fn verloc_stats(
    Query(output): Query<OutputParams>,
    State(metrics_state): State<MetricsAppState>,
) -> VerlocStatsResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(build_response(&metrics_state.verloc).await)
}

async fn build_response(verloc_stats: &SharedVerlocStats) -> VerlocStats {
    fn verloc_result_to_response(data: &nym_verloc::models::VerlocResultData) -> VerlocResultData {
        VerlocResultData {
            nodes_tested: data.nodes_tested,
            run_started: data.run_started,
            run_finished: data.run_finished,
            results: data
                .results
                .iter()
                .map(|r| VerlocNodeResult {
                    node_identity: r.node_identity,
                })
                .collect(),
        }
    }

    let guard = verloc_stats.read().await;

    let previous = if !guard.previous_run_data.run_finished() {
        VerlocResult::Unavailable
    } else {
        VerlocResult::Data(verloc_result_to_response(&guard.previous_run_data))
    };

    let current = if !guard.current_run_data.run_finished() {
        VerlocResult::MeasurementInProgress
    } else {
        VerlocResult::Data(verloc_result_to_response(&guard.previous_run_data))
    };

    VerlocStats { previous, current }
}

pub type VerlocStatsResponse = FormattedResponse<VerlocStats>;
