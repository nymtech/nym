// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::metrics::MetricsAppState;
use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_metrics::NymNodeMetrics;
use nym_node_requests::api::v1::metrics::models::LegacyMixingStats;

/// If applicable, returns mixing statistics information of this node.
/// This information is **PURELY** self-reported and in no way validated.
#[utoipa::path(
    get,
    path = "/mixing",
    context_path = "/api/v1/metrics",
    tag = "Metrics",
    responses(
        (status = 200, content(
            ("application/json" = LegacyMixingStats),
            ("application/yaml" = LegacyMixingStats)
        ))
    ),
    params(OutputParams),
)]
#[deprecated]
pub(crate) async fn legacy_mixing_stats(
    Query(output): Query<OutputParams>,
    State(metrics_state): State<MetricsAppState>,
) -> LegacyMixingStatsResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(build_legacy_response(&metrics_state.metrics))
}

fn build_legacy_response(metrics: &NymNodeMetrics) -> LegacyMixingStats {
    LegacyMixingStats {
        update_time: metrics.mixnet.legacy.last_update(),
        previous_update_time: metrics.mixnet.legacy.previous_update(),
        received_since_startup: metrics.mixnet.ingress.forward_hop_packets_received() as u64,
        sent_since_startup: metrics.mixnet.egress.forward_hop_packets_sent() as u64,
        dropped_since_startup: metrics.mixnet.egress.forward_hop_packets_dropped() as u64,
        received_since_last_update: metrics.mixnet.legacy.received_since_last_update() as u64,
        sent_since_last_update: metrics.mixnet.legacy.sent_since_last_update() as u64,
        dropped_since_last_update: metrics.mixnet.legacy.dropped_since_last_update() as u64,
    }
}

pub type LegacyMixingStatsResponse = FormattedResponse<LegacyMixingStats>;
