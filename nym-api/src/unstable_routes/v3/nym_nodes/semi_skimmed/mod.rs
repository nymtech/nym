// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumResult;
use crate::nym_nodes::node_view::NodeSubset;
use crate::support::http::state::AppState;
use crate::unstable_routes::helpers::build_nym_nodes_response;
use axum::extract::{Query, State};
use nym_api_requests::models::OffsetDateTimeJsonSchemaWrapper;
use nym_api_requests::nym_nodes::{PaginatedCachedNodesResponseV2, SemiSkimmedNodeV3};
use nym_api_requests::pagination::PaginatedResponse;
use nym_http_api_common::{FormattedResponse, OutputParams};
use utoipa::ToSchema;

pub type PaginatedSemiSkimmedNodes =
    AxumResult<FormattedResponse<PaginatedCachedNodesResponseV2<SemiSkimmedNodeV3>>>;

#[allow(dead_code)] // not dead, used in OpenAPI docs
#[derive(ToSchema)]
#[schema(title = "PaginatedCachedNodesExpandedV3ResponseSchema")]
pub struct PaginatedCachedNodesExpandedV3ResponseSchema {
    pub refreshed_at: OffsetDateTimeJsonSchemaWrapper,
    #[schema(value_type = SemiSkimmedNodeV3)]
    pub nodes: PaginatedResponse<SemiSkimmedNodeV3>,
}

/// Return all Nym Nodes that are currently bonded.
#[utoipa::path(
    operation_id = "v3_nodes_expanded",
    tag = "Unstable Nym Nodes v3",
    get,
    params(OutputParams),
    path = "/semi-skimmed",
    context_path = "/v3/unstable/nym-nodes",
    responses(
        (status = 200, content(
            (PaginatedCachedNodesExpandedV3ResponseSchema = "application/json"),
            (PaginatedCachedNodesExpandedV3ResponseSchema = "application/yaml"),
            (PaginatedCachedNodesExpandedV3ResponseSchema = "application/bincode")
        ))
    )
)]
pub(super) async fn nodes_expanded(
    state: State<AppState>,
    Query(output): Query<OutputParams>,
) -> PaginatedSemiSkimmedNodes {
    let snapshot = state.latest_snapshot().await?;
    let mixnet = snapshot.mixnet();
    let nodes = build_nym_nodes_response(&snapshot, NodeSubset::All, false);

    Ok(output.to_response(PaginatedCachedNodesResponseV2::new_full(
        mixnet.current_interval.current_epoch_absolute_id(),
        mixnet.current_key_rotation_id(),
        snapshot.block_time(),
        nodes,
    )))
}
