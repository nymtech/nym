// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::router::api::{FormattedResponse, OutputParams};
use axum::extract::Query;
use axum::http::StatusCode;
use nym_node_requests::api::v1::gateway::models::Gateway;

/// Returns root gateway information
#[utoipa::path(
    get,
    path = "",
    context_path = "/api/v1/gateway",
    tag = "Gateway",
    responses(
        (status = 501, description = "the endpoint hasn't been implemented yet"),
        (status = 200, content(
            ("application/json" = Gateway),
            ("application/yaml" = Gateway)
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn root_gateway(
    details: Option<Gateway>,
    Query(output): Query<OutputParams>,
) -> Result<GatewayResponse, StatusCode> {
    let details = details.ok_or(StatusCode::NOT_IMPLEMENTED)?;
    let output = output.output.unwrap_or_default();
    Ok(output.to_response(details))
}

pub type GatewayResponse = FormattedResponse<Gateway>;
