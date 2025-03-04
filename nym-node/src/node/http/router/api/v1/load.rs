// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::AppState;
use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_requests::api::v1::node_load::models::NodeLoad;

/// Returns current relative load this node.
#[utoipa::path(
    get,
    path = "/load",
    context_path = "/api/v1",
    tag = "Node",
    responses(
        (status = 200, content(
            (NodeLoad = "application/json"),
            (NodeLoad = "application/yaml")
        ),  description = "current node load")
    ),
    params(OutputParams)
)]
pub(crate) async fn root_load(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> NodeLoadResponse {
    let output = output.output.unwrap_or_default();
    let load = state.cached_load.current_load();

    output.to_response(load)
}

pub type NodeLoadResponse = FormattedResponse<NodeLoad>;
