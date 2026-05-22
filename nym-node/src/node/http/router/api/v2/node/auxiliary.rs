// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::router::types::RequestError;
use crate::node::http::state::AppState;
use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParamsV2};
use nym_node_requests::api::v2::node::models::AuxiliaryDetailsV2;

/// Returns auxiliary details of this node.
#[utoipa::path(
    get,
    path = "/auxiliary-details",
    context_path = "/api/v2",
    tag = "v2 / Node",
    // distinct from v1's `auxiliary`: OpenAPI requires operationId to be unique
    // across the whole document, and Swagger UI routes "Try it out" by operationId
    operation_id = "v2_auxiliary",
    responses(
        (status = 200, content(
            (AuxiliaryDetailsV2 = "application/json"),
            (AuxiliaryDetailsV2 = "application/yaml")
        )),
    ),
    params(OutputParamsV2)
)]
pub(crate) async fn auxiliary(
    Query(output): Query<OutputParamsV2>,
    State(state): State<AppState>,
) -> Result<AuxiliaryDetailsResponse, RequestError> {
    let output = output.output.unwrap_or_default();
    Ok(output.to_response(state.static_information.auxiliary_data.clone()))
}

pub type AuxiliaryDetailsResponse = FormattedResponse<AuxiliaryDetailsV2>;
