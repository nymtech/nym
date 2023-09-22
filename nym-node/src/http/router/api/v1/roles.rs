// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::router::api::{FormattedResponse, OutputParams};
use axum::extract::Query;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Debug, Copy, ToSchema, Serialize)]
pub struct NodeRoles {
    pub mixnode_enabled: bool,
    pub gateway_enabled: bool,
    pub network_requester_enabled: bool,
}

/// Returns roles supported by this node
#[utoipa::path(
    get,
    path = "/roles",
    context_path = "/api/v1",
    tag = "Base",
    responses(
        (status = 200, content(
            ("application/json" = NodeRoles),
            ("application/yaml" = NodeRoles)
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn roles(
    node_roles: NodeRoles,
    Query(output): Query<OutputParams>,
) -> RolesResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(node_roles)
}

pub type RolesResponse = FormattedResponse<NodeRoles>;
