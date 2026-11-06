// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumResult;
use crate::support::attested_snapshots::data::RetainedHeight;
use crate::support::http::state::AppState;
use crate::unstable_routes::helpers::refreshed_at;
use axum::extract::{Query, State};
use nym_api_requests::models::described::v3::NymNodeDescriptionV3;
use nym_api_requests::models::OffsetDateTimeJsonSchemaWrapper;
use nym_api_requests::nym_nodes::{NodeRole, PaginatedCachedNodesResponseV2, SemiSkimmedNodeV3};
use nym_api_requests::pagination::PaginatedResponse;
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_topology::CachedEpochRewardedSet;
use utoipa::ToSchema;

pub type PaginatedSemiSkimmedNodes =
    AxumResult<FormattedResponse<PaginatedCachedNodesResponseV2<SemiSkimmedNodeV3>>>;

fn build_response<'a>(
    rewarded_set: &CachedEpochRewardedSet,
    nym_nodes: impl Iterator<Item = &'a NymNodeDescriptionV3>,
    snapshot: &RetainedHeight,
    current_key_rotation: u32,
) -> Vec<SemiSkimmedNodeV3> {
    let mut nodes = Vec::new();
    for nym_node in nym_nodes {
        let node_id = nym_node.node_id;

        let role: NodeRole = rewarded_set.role(node_id).into();

        nodes.push(nym_node.to_semi_skimmed_node_v3(
            current_key_rotation,
            role,
            snapshot.published_performance(node_id),
        ));
    }
    nodes
}

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
    // 1. grab all relevant described nym-nodes
    let rewarded_set = state.rewarded_set().await?;

    let describe_cache = state.describe_nodes_cache_data().await?;
    let all_nym_nodes = describe_cache.all_nym_nodes();
    let snapshot = state.latest_snapshot().await?;

    let contract_cache = state.nym_contract_cache();
    let current_key_rotation = contract_cache.current_key_rotation_id().await?;
    let interval = contract_cache.current_interval().await?;

    let nodes = build_response(
        &rewarded_set,
        all_nym_nodes,
        &snapshot,
        current_key_rotation,
    );

    // min of all caches
    let refreshed_at = refreshed_at([
        rewarded_set.timestamp(),
        snapshot.performance_auxiliary.block_time(),
        describe_cache.timestamp(),
    ]);

    Ok(output.to_response(PaginatedCachedNodesResponseV2::new_full(
        interval.current_epoch_absolute_id(),
        current_key_rotation,
        refreshed_at,
        nodes,
    )))
}
