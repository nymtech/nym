// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::AppState;
use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_requests::api::v1::node::models::NodeRoles;

/// Returns roles supported by this node
#[utoipa::path(
    get,
    path = "/roles",
    context_path = "/api/v1",
    tag = "v1 / Node",
    responses(
        (status = 200, content(
            (NodeRoles = "application/json"),
            (NodeRoles = "application/yaml")
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn roles(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> RolesResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(state.static_information.roles)
}

pub type RolesResponse = FormattedResponse<NodeRoles>;
