// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumResult;
use crate::nym_nodes::node_view::NodeSubset;
use crate::support::http::state::AppState;
use crate::unstable_routes::helpers::build_nym_nodes_response;
use crate::unstable_routes::v2::nym_nodes::helpers::NodesParamsWithRole;
use axum::extract::{Query, State};
use nym_api_requests::models::OffsetDateTimeJsonSchemaWrapper;
use nym_api_requests::nym_nodes::{PaginatedCachedNodesResponseV2, SemiSkimmedNodeV1};
use nym_api_requests::pagination::PaginatedResponse;
use nym_http_api_common::FormattedResponse;
use utoipa::ToSchema;

pub type PaginatedSemiSkimmedNodes =
    AxumResult<FormattedResponse<PaginatedCachedNodesResponseV2<SemiSkimmedNodeV1>>>;

#[allow(dead_code)] // not dead, used in OpenAPI docs
#[derive(ToSchema)]
#[schema(title = "PaginatedCachedNodesExpandedV2ResponseSchema")]
pub struct PaginatedCachedNodesExpandedV2ResponseSchema {
    pub refreshed_at: OffsetDateTimeJsonSchemaWrapper,
    #[schema(value_type = SemiSkimmedNodeV1)]
    pub nodes: PaginatedResponse<SemiSkimmedNodeV1>,
}

/// Return all Nym Nodes and optionally legacy mixnodes/gateways (if `no-legacy` flag is not used)
/// that are currently bonded.
#[utoipa::path(
    operation_id = "v2_nodes_expanded",
    tag = "Unstable Nym Nodes v2",
    get,
    params(NodesParamsWithRole),
    path = "",
    context_path = "/v2/unstable/nym-nodes/semi-skimmed",
    responses(
        (status = 200, content(
            (PaginatedCachedNodesExpandedV2ResponseSchema = "application/json"),
            (PaginatedCachedNodesExpandedV2ResponseSchema = "application/yaml"),
            (PaginatedCachedNodesExpandedV2ResponseSchema = "application/bincode")
        ))
    )
)]
pub(super) async fn nodes_expanded(
    state: State<AppState>,
    query_params: Query<NodesParamsWithRole>,
) -> PaginatedSemiSkimmedNodes {
    let snapshot = state.latest_snapshot().await?;
    let mixnet = snapshot.mixnet();
    let nodes = build_nym_nodes_response(&snapshot, NodeSubset::All, false);

    let output = query_params.output.unwrap_or_default();
    Ok(output.to_response(PaginatedCachedNodesResponseV2::new_full(
        mixnet.current_interval.current_epoch_absolute_id(),
        mixnet.current_key_rotation_id(),
        snapshot.block_time(),
        nodes,
    )))
}
