// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::state::AppState;
use crate::unstable_routes::v1::nym_nodes::helpers::NodesParamsWithRole;
use axum::extract::{Query, State};
use nym_api_requests::nym_nodes::{CachedNodesResponse, FullFatNode};
use nym_http_api_common::FormattedResponse;

#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParamsWithRole),
    path = "",
    context_path = "/v1/unstable/nym-nodes/full-fat",
    responses(
        // (status = 200, body = CachedNodesResponse<FullFatNode>)
        (status = 501)
    )
)]
pub(crate) async fn nodes_detailed(
    _state: State<AppState>,
    _query_params: Query<NodesParamsWithRole>,
) -> AxumResult<FormattedResponse<CachedNodesResponse<FullFatNode>>> {
    Err(AxumErrorResponse::not_implemented())
}
