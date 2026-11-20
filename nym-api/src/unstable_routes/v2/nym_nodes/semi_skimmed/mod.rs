// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumResult;
use crate::support::attested_snapshots::data::RetainedHeight;
use crate::support::http::state::AppState;
use crate::unstable_routes::helpers::refreshed_at;
use crate::unstable_routes::v2::nym_nodes::helpers::NodesParamsWithRole;
use axum::extract::{Query, State};
use nym_api_requests::models::described::v3::NymNodeDescriptionV3;
use nym_api_requests::models::OffsetDateTimeJsonSchemaWrapper;
use nym_api_requests::nym_nodes::{NodeRole, PaginatedCachedNodesResponseV2, SemiSkimmedNodeV1};
use nym_api_requests::pagination::PaginatedResponse;
use nym_http_api_common::FormattedResponse;
use nym_topology::CachedEpochRewardedSet;
use utoipa::ToSchema;

pub type PaginatedSemiSkimmedNodes =
    AxumResult<FormattedResponse<PaginatedCachedNodesResponseV2<SemiSkimmedNodeV1>>>;

//SW TODO : this is copied from skimmed nodes, surely we can do better than that
fn build_nym_nodes_response<'a, NI>(
    rewarded_set: &CachedEpochRewardedSet,
    nym_nodes_subset: NI,
    snapshot: &RetainedHeight,
    current_key_rotation: u32,
    active_only: bool,
) -> Vec<SemiSkimmedNodeV1>
where
    NI: Iterator<Item = &'a NymNodeDescriptionV3> + 'a,
{
    let mut nodes = Vec::new();
    for nym_node in nym_nodes_subset {
        let node_id = nym_node.node_id;

        let role: NodeRole = rewarded_set.role(node_id).into();

        // if the role is inactive, see if our filter allows it
        if active_only && role.is_inactive() {
            continue;
        }

        nodes.push(nym_node.to_semi_skimmed_node(
            current_key_rotation,
            role,
            snapshot.published_performance(node_id),
        ));
    }
    nodes
}

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
    // 1. grab all relevant described nym-nodes
    let rewarded_set = state.rewarded_set().await?;

    let describe_cache = state.describe_nodes_cache_data().await?;
    let all_nym_nodes = describe_cache.all_nym_nodes();
    let snapshot = state.latest_snapshot().await?;

    let contract_cache = state.nym_contract_cache();
    let current_key_rotation = contract_cache.current_key_rotation_id().await?;
    let interval = contract_cache.current_interval().await?;

    let nodes = build_nym_nodes_response(
        &rewarded_set,
        all_nym_nodes,
        &snapshot,
        current_key_rotation,
        false,
    );

    // min of all caches
    let refreshed_at = refreshed_at([
        rewarded_set.timestamp(),
        snapshot.performance_auxiliary.block_time(),
        describe_cache.timestamp(),
    ]);

    let output = query_params.output.unwrap_or_default();
    Ok(output.to_response(PaginatedCachedNodesResponseV2::new_full(
        interval.current_epoch_absolute_id(),
        current_key_rotation,
        refreshed_at,
        nodes,
    )))
}
