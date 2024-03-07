// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::node_statistics::{NodeStatsSimple, SharedNodeStats};
use axum::extract::{Query, State};
use nym_node::http::api::{FormattedResponse, Output};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(untagged)]
pub enum NodeStatsResponse {
    Full(String),
    Simple(NodeStatsSimple),
}

pub(crate) async fn metrics(
    Query(_params): Query<StatsQueryParams>,
    State(stats): State<SharedNodeStats>,
) -> String {
    let response = generate_stats(true, stats).await;
    match response {
        NodeStatsResponse::Full(full) => full,
        NodeStatsResponse::Simple(_) => unreachable!(),
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
        NodeStatsResponse::Full(snapshot_data.prom().await)
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
