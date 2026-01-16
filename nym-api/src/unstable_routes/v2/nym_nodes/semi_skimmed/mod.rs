// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumResult;
use crate::support::http::state::AppState;
use crate::unstable_routes::helpers::refreshed_at;
use crate::unstable_routes::v2::nym_nodes::helpers::NodesParamsWithRole;
use axum::extract::{Query, State};
use nym_api_requests::models::{
    NodeAnnotation, NymNodeDescription, OffsetDateTimeJsonSchemaWrapper,
};
use nym_api_requests::nym_nodes::{NodeRole, PaginatedCachedNodesResponseV2, SemiSkimmedNode};
use nym_api_requests::pagination::PaginatedResponse;
use nym_http_api_common::FormattedResponse;
use nym_mixnet_contract_common::NodeId;
use nym_topology::CachedEpochRewardedSet;
use std::collections::HashMap;
use utoipa::ToSchema;

pub type PaginatedSemiSkimmedNodes =
    AxumResult<FormattedResponse<PaginatedCachedNodesResponseV2<SemiSkimmedNode>>>;

//SW TODO : this is copied from skimmed nodes, surely we can do better than that
fn build_nym_nodes_response<'a, NI>(
    rewarded_set: &CachedEpochRewardedSet,
    nym_nodes_subset: NI,
    annotations: &HashMap<NodeId, NodeAnnotation>,
    current_key_rotation: u32,
    active_only: bool,
) -> Vec<SemiSkimmedNode>
where
    NI: Iterator<Item = &'a NymNodeDescription> + 'a,
{
    let mut nodes = Vec::new();
    for nym_node in nym_nodes_subset {
        let node_id = nym_node.node_id;

        let role: NodeRole = rewarded_set.role(node_id).into();

        // if the role is inactive, see if our filter allows it
        if active_only && role.is_inactive() {
            continue;
        }

        // honestly, not sure under what exact circumstances this value could be missing,
        // but in that case just use 0 performance
        let annotation = annotations.get(&node_id).copied().unwrap_or_default();

        nodes.push(nym_node.to_semi_skimmed_node(
            current_key_rotation,
            role,
            annotation.last_24h_performance,
        ));
    }
    nodes
}

#[allow(dead_code)] // not dead, used in OpenAPI docs
#[derive(ToSchema)]
#[schema(title = "PaginatedCachedNodesExpandedResponseSchema")]
pub struct PaginatedCachedNodesExpandedResponseSchema {
    pub refreshed_at: OffsetDateTimeJsonSchemaWrapper,
    #[schema(value_type = SemiSkimmedNode)]
    pub nodes: PaginatedResponse<SemiSkimmedNode>,
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
            (PaginatedCachedNodesExpandedResponseSchema = "application/json"),
            (PaginatedCachedNodesExpandedResponseSchema = "application/yaml"),
            (PaginatedCachedNodesExpandedResponseSchema = "application/bincode")
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
    let status_cache = &state.node_status_cache();
    let annotations = status_cache.node_annotations().await?;

    let contract_cache = state.nym_contract_cache();
    let current_key_rotation = contract_cache.current_key_rotation_id().await?;
    let interval = contract_cache.current_interval().await?;

    let nodes = build_nym_nodes_response(
        &rewarded_set,
        all_nym_nodes,
        &annotations,
        current_key_rotation,
        false,
    );

    // min of all caches
    let refreshed_at = refreshed_at([
        rewarded_set.timestamp(),
        status_cache.cache_timestamp().await,
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
