// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use nym_api_requests::nym_nodes::{NodesByAddressesRequestBody, NodesByAddressesResponse};
use nym_http_api_common::{FormattedResponse, OutputParams};
use std::collections::HashMap;

#[utoipa::path(
    tag = "Unstable Nym Nodes",
    post,
    request_body = NodesByAddressesRequestBody,
    path = "/by-addresses",
    context_path = "/v1/unstable/nym-nodes",
    responses(
        (status = 200, content(
            (NodesByAddressesResponse = "application/json"),
            (NodesByAddressesResponse = "application/yaml"),
            (NodesByAddressesResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn nodes_by_addresses(
    Query(output): Query<OutputParams>,
    state: State<AppState>,
    Json(body): Json<NodesByAddressesRequestBody>,
) -> AxumResult<FormattedResponse<NodesByAddressesResponse>> {
    // if the request is too big, simply reject it
    if body.addresses.len() > 100 {
        return Err(AxumErrorResponse::bad_request(
            "requested too many addresses",
        ));
    }

    let output = output.output.unwrap_or_default();

    // TODO: perhaps introduce different cache because realistically nym-api will receive
    // request for the same couple addresses from all nodes in quick succession
    let describe_cache = state.describe_nodes_cache_data().await?;

    let mut existence = HashMap::new();
    for address in body.addresses {
        existence.insert(address, describe_cache.node_with_address(address));
    }

    Ok(output.to_response(NodesByAddressesResponse { existence }))
}
