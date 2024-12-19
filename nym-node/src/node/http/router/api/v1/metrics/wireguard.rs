// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::metrics::MetricsAppState;
use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_metrics::NymNodeMetrics;
use nym_node_requests::api::v1::metrics::models::WireguardStats;

/// If applicable, returns wireguard statistics information of this node.
/// This information is **PURELY** self-reported and in no way validated.
#[utoipa::path(
    get,
    path = "/wireguard-stats",
    context_path = "/api/v1/metrics",
    tag = "Metrics",
    responses(
        (status = 200, content(
            (WireguardStats = "application/json"),
            (WireguardStats = "application/yaml")
        ))
    ),
    params(OutputParams),
)]
pub(crate) async fn wireguard_stats(
    Query(output): Query<OutputParams>,
    State(metrics_state): State<MetricsAppState>,
) -> WireguardStatsResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(build_response(&metrics_state.metrics))
}

fn build_response(metrics: &NymNodeMetrics) -> WireguardStats {
    WireguardStats {
        bytes_tx: metrics.wireguard.bytes_tx(),
        bytes_rx: metrics.wireguard.bytes_rx(),
    }
}

pub type WireguardStatsResponse = FormattedResponse<WireguardStats>;
