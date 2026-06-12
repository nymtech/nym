// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::router::types::RequestError;
use crate::node::http::state::AppState;
use axum::extract::{Query, State};
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_requests::api::v1::node::models::AuxiliaryDetailsV1;

/// Returns auxiliary details of this node.
#[utoipa::path(
    get,
    path = "/auxiliary-details",
    context_path = "/api/v1",
    tag = "v1 / Node",
    responses(
        (status = 200, content(
            (AuxiliaryDetailsV1 = "application/json"),
            (AuxiliaryDetailsV1 = "application/yaml")
        )),
    ),
    params(OutputParams)
)]
pub(crate) async fn auxiliary(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> Result<AuxiliaryDetailsResponse, RequestError> {
    let output = output.output.unwrap_or_default();
    Ok(output.to_response(state.static_information.auxiliary_data.clone().into()))
}

pub type AuxiliaryDetailsResponse = FormattedResponse<AuxiliaryDetailsV1>;
