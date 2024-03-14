// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::node_statistics::{NodeStatsSimple, SharedNodeStats};

use axum::{
    extract::{Query, State},
    http::HeaderMap,
};
use nym_http_api_common::{FormattedResponse, Output};
use nym_metrics::metrics;
use serde::{Deserialize, Serialize};

use super::state::MixnodeAppState;

#[derive(Serialize)]
#[serde(untagged)]
pub enum NodeStatsResponse {
    Full(String),
    Simple(NodeStatsSimple),
}

pub(crate) async fn metrics(State(state): State<MixnodeAppState>, headers: HeaderMap) -> String {
    if let Some(metrics_key) = state.metrics_key {
        if let Some(auth) = headers.get("Authorization") {
            if auth.to_str().unwrap_or_default() == format!("Bearer {}", metrics_key) {
                metrics!()
            } else {
                "Unauthorized".to_string()
            }
        } else {
            "Unauthorized".to_string()
        }
    } else {
        "Set metrics_key in config to enable Prometheus metrics".to_string()
    }
}

pub(crate) async fn stats(
    Query(params): Query<StatsQueryParams>,
    State(stats): State<SharedNodeStats>,
) -> MixnodeStatsResponse {
    let output = params.output.unwrap_or_default();

    // there's no point in returning the entire hashmap of sending destinations in regular mode
    let response = generate_stats(params.debug, stats).await;
    output.to_response(response)
}

async fn generate_stats(full: bool, stats: SharedNodeStats) -> NodeStatsResponse {
    let snapshot_data = stats.clone_data().await;
    if full {
        NodeStatsResponse::Full(metrics!())
    } else {
        NodeStatsResponse::Simple(snapshot_data.simplify())
    }
}

pub type MixnodeStatsResponse = FormattedResponse<NodeStatsResponse>;

#[derive(Default, Debug, Serialize, Deserialize, Copy, Clone)]
// #[derive(Default, Debug, Serialize, Deserialize, Copy, Clone, IntoParams, ToSchema)]
#[serde(default)]
pub(crate) struct StatsQueryParams {
    debug: bool,
    pub output: Option<Output>,
}
