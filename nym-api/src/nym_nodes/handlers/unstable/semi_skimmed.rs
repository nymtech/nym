// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::nym_nodes::handlers::unstable::NodesParamsWithRole;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use nym_api_requests::nym_nodes::{CachedNodesResponse, SemiSkimmedNode};

#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParamsWithRole),
    path = "",
    context_path = "/v1/unstable/nym-nodes/semi-skimmed",
    responses(
    // (status = 200, body = CachedNodesResponse<SemiSkimmedNode>)
        (status = 501)
    )
)]
pub(super) async fn nodes_expanded(
    _state: State<AppState>,
    _query_params: Query<NodesParamsWithRole>,
) -> AxumResult<Json<CachedNodesResponse<SemiSkimmedNode>>> {
    Err(AxumErrorResponse::not_implemented())
}
