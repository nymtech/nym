// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::extract::Query;
use axum::http::StatusCode;
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_node_requests::api::v1::mixnode::models::Mixnode;

/// Returns root mixnode information
#[utoipa::path(
    get,
    path = "",
    context_path = "/api/v1/mixnode",
    tag = "Mixnode",
    responses(
        (status = 501, description = "the endpoint hasn't been implemented yet"),
        (status = 200, content(
            (Mixnode = "application/json"),
            (Mixnode = "application/yaml")
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn root_mixnode(
    details: Option<Mixnode>,
    Query(output): Query<OutputParams>,
) -> Result<MixnodeResponse, StatusCode> {
    let details = details.ok_or(StatusCode::NOT_IMPLEMENTED)?;
    let output = output.output.unwrap_or_default();
    Ok(output.to_response(details))
}

pub type MixnodeResponse = FormattedResponse<Mixnode>;
