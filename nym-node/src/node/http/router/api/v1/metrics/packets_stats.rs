// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::metrics::MetricsAppState;
use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_metrics::NymNodeMetrics;
use nym_node_requests::api::v1::metrics::models::packets::{
    EgressMixingStats, IngressMixingStats, PacketsStats,
};

/// If applicable, returns packets statistics information of this node.
/// This information is **PURELY** self-reported and in no way validated.
#[utoipa::path(
    get,
    path = "/packets-stats",
    context_path = "/api/v1/metrics",
    tag = "Metrics",
    responses(
        (status = 200, content(
            (PacketsStats = "application/json"),
            (PacketsStats = "application/yaml")
        ))
    ),
    params(OutputParams),
)]
pub(crate) async fn packets_stats(
    Query(output): Query<OutputParams>,
    State(metrics_state): State<MetricsAppState>,
) -> PacketsStatsResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(build_response(&metrics_state.metrics))
}

fn build_response(metrics: &NymNodeMetrics) -> PacketsStats {
    PacketsStats {
        ingress_mixing: IngressMixingStats {
            forward_hop_packets_received: metrics.mixnet.ingress.forward_hop_packets_received(),
            final_hop_packets_received: metrics.mixnet.ingress.final_hop_packets_received(),
            malformed_packets_received: metrics.mixnet.ingress.malformed_packets_received(),
            excessive_delay_packets: metrics.mixnet.ingress.excessive_delay_packets(),
            forward_hop_packets_dropped: metrics.mixnet.ingress.forward_hop_packets_dropped(),
            final_hop_packets_dropped: metrics.mixnet.ingress.final_hop_packets_dropped(),
        },
        egress_mixing: EgressMixingStats {
            forward_hop_packets_sent: metrics.mixnet.egress.forward_hop_packets_sent(),
            forward_hop_packets_dropped: metrics.mixnet.egress.forward_hop_packets_dropped(),
            ack_packets_sent: metrics.mixnet.egress.ack_packets_sent(),
        },
    }
}

pub type PacketsStatsResponse = FormattedResponse<PacketsStats>;
