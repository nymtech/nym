// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::api::{FormattedResponse, OutputParams};
use crate::router::types::RequestError;
use axum::extract::Query;
use nym_node_requests::api::v1::node::models::AuxiliaryDetails;

/// Returns auxiliary details of this node.
#[utoipa::path(
    get,
    path = "/auxiliary-details",
    context_path = "/api/v1",
    tag = "Node",
    responses(
        (status = 200, content(
            ("application/json" = AuxiliaryDetails),
            ("application/yaml" = AuxiliaryDetails)
        )),
    ),
    params(OutputParams)
)]
pub(crate) async fn auxiliary(
    description: AuxiliaryDetails,
    Query(output): Query<OutputParams>,
) -> Result<AuxiliaryDetailsResponse, RequestError> {
    let output = output.output.unwrap_or_default();
    Ok(output.to_response(description))
}

pub type AuxiliaryDetailsResponse = FormattedResponse<AuxiliaryDetails>;
