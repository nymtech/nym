// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::api::{FormattedResponse, OutputParams};
use crate::router::types::RequestError;
use axum::extract::Query;
use nym_node_requests::api::v1::node::models::NodeDescription;

/// Returns human-readable description of this node.
#[utoipa::path(
    get,
    path = "/description",
    context_path = "/api/v1",
    tag = "Node",
    responses(
        (status = 200, content(
            ("application/json" = NodeDescription),
            ("application/yaml" = NodeDescription)
        )),
    ),
    params(OutputParams)
)]
pub(crate) async fn description(
    description: NodeDescription,
    Query(output): Query<OutputParams>,
) -> Result<NodeDescriptionResponse, RequestError> {
    let output = output.output.unwrap_or_default();
    Ok(output.to_response(description))
}

pub type NodeDescriptionResponse = FormattedResponse<NodeDescription>;
