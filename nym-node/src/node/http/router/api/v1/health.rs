// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::AppState;
use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_requests::api::v1::health::models::NodeHealth;

/// Returns health status of this node.
#[utoipa::path(
    get,
    path = "/health",
    context_path = "/api/v1",
    tag = "Health",
    responses(
        (status = 200, content(
            (NodeHealth = "application/json"),
            (NodeHealth = "application/yaml")
        ),  description = "the api is available and healthy")
    ),
    params(OutputParams)
)]
pub(crate) async fn root_health(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> HealthResponse {
    let output = output.output.unwrap_or_default();
    let uptime = state.startup_time.elapsed();
    let health = NodeHealth::new_healthy(uptime);

    output.to_response(health)
}

pub type HealthResponse = FormattedResponse<NodeHealth>;
