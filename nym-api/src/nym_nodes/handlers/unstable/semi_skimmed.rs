// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_describe_cache::DescribedNodes;
use crate::node_status_api::models::AxumResult;
use crate::nym_nodes::handlers::unstable::helpers::{refreshed_at, LegacyAnnotation};
use crate::nym_nodes::handlers::unstable::NodesParamsWithRole;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use nym_api_requests::models::{
    NodeAnnotation, NymNodeDescription, OffsetDateTimeJsonSchemaWrapper,
};
use nym_api_requests::nym_nodes::{NodeRole, PaginatedCachedNodesResponse, SemiSkimmedNode};
use nym_api_requests::pagination::PaginatedResponse;
use nym_http_api_common::FormattedResponse;
use nym_mixnet_contract_common::NodeId;
use nym_topology::CachedEpochRewardedSet;
use std::collections::HashMap;
use tracing::trace;
use utoipa::ToSchema;

pub type PaginatedSemiSkimmedNodes =
    AxumResult<FormattedResponse<PaginatedCachedNodesResponse<SemiSkimmedNode>>>;

//SW TODO : this is copied from skimmed nodes, surely we can do better than that
fn build_nym_nodes_response<'a, NI>(
    rewarded_set: &CachedEpochRewardedSet,
    nym_nodes_subset: NI,
    annotations: &HashMap<NodeId, NodeAnnotation>,
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

        nodes.push(nym_node.to_semi_skimmed_node(role, annotation.last_24h_performance));
    }
    nodes
}

//SW TODO : this is copied from skimmed nodes, surely we can do better than that
/// Given all relevant caches, add appropriate legacy nodes to the part of the response
fn add_legacy<LN>(
    nodes: &mut Vec<SemiSkimmedNode>,
    rewarded_set: &CachedEpochRewardedSet,
    describe_cache: &DescribedNodes,
    annotated_legacy_nodes: &HashMap<NodeId, LN>,
) where
    LN: LegacyAnnotation,
{
    for (node_id, legacy) in annotated_legacy_nodes.iter() {
        let role: NodeRole = rewarded_set.role(*node_id).into();

        // if we have self-described info, prefer it over contract data
        if let Some(described) = describe_cache.get_node(node_id) {
            nodes.push(described.to_semi_skimmed_node(role, legacy.performance()))
        } else {
            match legacy.try_to_semi_skimmed_node(role) {
                Ok(node) => nodes.push(node),
                Err(err) => {
                    let id = legacy.identity();
                    trace!("node {id} is malformed: {err}")
                }
            }
        }
    }
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
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParamsWithRole),
    path = "",
    context_path = "/v1/unstable/nym-nodes/semi-skimmed",
    responses(
        (status = 200, body = PaginatedCachedNodesExpandedResponseSchema)
    )
)]
pub(super) async fn nodes_expanded(
    state: State<AppState>,
    _query_params: Query<NodesParamsWithRole>,
) -> PaginatedSemiSkimmedNodes {
    // 1. grab all relevant described nym-nodes
    let rewarded_set = state.rewarded_set().await?;

    let describe_cache = state.describe_nodes_cache_data().await?;
    let all_nym_nodes = describe_cache.all_nym_nodes();
    let annotations = state.node_annotations().await?;
    let legacy_mixnodes = state.legacy_mixnode_annotations().await?;
    let legacy_gateways = state.legacy_gateways_annotations().await?;

    let mut nodes = build_nym_nodes_response(&rewarded_set, all_nym_nodes, &annotations, false);

    // add legacy gateways to the response
    add_legacy(&mut nodes, &rewarded_set, &describe_cache, &legacy_gateways);

    // add legacy mixnodes to the response
    add_legacy(&mut nodes, &rewarded_set, &describe_cache, &legacy_mixnodes);

    // min of all caches
    let refreshed_at = refreshed_at([
        rewarded_set.timestamp(),
        annotations.timestamp(),
        describe_cache.timestamp(),
        legacy_mixnodes.timestamp(),
        legacy_gateways.timestamp(),
    ]);

    Ok(Json(PaginatedCachedNodesResponse::new_full(
        refreshed_at,
        nodes,
    )))
}
