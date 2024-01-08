// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::router::api::{FormattedResponse, OutputParams};
use axum::extract::Query;
use axum::http::StatusCode;
use nym_node_requests::api::v1::network_requester::models::NetworkRequester;

/// Returns root network requester information
#[utoipa::path(
    get,
    path = "",
    context_path = "/api/v1/network-requester",
    tag = "Network Requester",
    responses(
        (status = 501, description = "the endpoint hasn't been implemented yet"),
        (status = 200, content(
            ("application/json" = NetworkRequester),
            ("application/yaml" = NetworkRequester)
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn root_network_requester(
    details: Option<NetworkRequester>,
    Query(output): Query<OutputParams>,
) -> Result<NetworkRequesterResponse, StatusCode> {
    let details = details.ok_or(StatusCode::NOT_IMPLEMENTED)?;
    let output = output.output.unwrap_or_default();
    Ok(output.to_response(details))
}

pub type NetworkRequesterResponse = FormattedResponse<NetworkRequester>;
