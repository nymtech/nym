// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::node_statistics::{NodeStats, NodeStatsSimple, SharedNodeStats};
use axum::extract::{Query, State};
use nym_node::http::api::{FormattedResponse, Output};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(untagged)]
pub enum NodeStatsResponse {
    Full(NodeStats),
    Simple(NodeStatsSimple),
}

pub(crate) async fn stats(
    Query(params): Query<StatsQueryParams>,
    State(stats): State<SharedNodeStats>,
) -> MixnodeStatsResponse {
    let output = params.output.unwrap_or_default();

    let snapshot_data = stats.clone_data().await;

    // there's no point in returning the entire hashmap of sending destinations in regular mode
    let response = if params.debug {
        NodeStatsResponse::Full(snapshot_data)
    } else {
        NodeStatsResponse::Simple(snapshot_data.simplify())
    };
    output.to_response(response)
}

pub type MixnodeStatsResponse = FormattedResponse<NodeStatsResponse>;

#[derive(Default, Debug, Serialize, Deserialize, Copy, Clone)]
// #[derive(Default, Debug, Serialize, Deserialize, Copy, Clone, IntoParams, ToSchema)]
#[serde(default)]
pub(crate) struct StatsQueryParams {
    debug: bool,
    pub output: Option<Output>,
}
