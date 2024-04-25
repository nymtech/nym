// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::api::{FormattedResponse, OutputParams};
use crate::router::types::RequestError;
use axum::extract::Query;
use axum::http::StatusCode;
use nym_node_requests::api::v1::node::models::HostSystem;

/// Returns build system information of the host running the binary.
#[utoipa::path(
    get,
    path = "/system-info",
    context_path = "/api/v1",
    tag = "Node",
    responses(
        (status = 200, content(
            ("application/json" = HostSystem),
            ("application/yaml" = HostSystem)
        )),
        (status = 403, body = ErrorResponse, description = "the node does not wish to expose the system information")
    ),
    params(OutputParams)
)]
pub(crate) async fn host_system(
    system_info: Option<HostSystem>,
    Query(output): Query<OutputParams>,
) -> Result<HostSystemResponse, RequestError> {
    let output = output.output.unwrap_or_default();

    let Some(system_info) = system_info else {
        return Err(RequestError::new(
            "this nym-node does not wish to expose the system information",
            StatusCode::FORBIDDEN,
        ));
    };

    Ok(output.to_response(system_info))
}

pub type HostSystemResponse = FormattedResponse<HostSystem>;
