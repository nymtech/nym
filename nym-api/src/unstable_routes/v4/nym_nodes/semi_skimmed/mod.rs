// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumResult;
use crate::support::http::state::AppState;
use crate::unstable_routes::helpers::refreshed_at;
use axum::extract::{Query, State};
use nym_api_requests::models::described::v3::NymNodeDescriptionV3;
use nym_api_requests::models::{
    NodeAnnotationV1, NodeAnnotationV2, OffsetDateTimeJsonSchemaWrapper,
};
use nym_api_requests::nym_nodes::{NodeRole, PaginatedCachedNodesResponseV2, SemiSkimmedNodeV4};
use nym_api_requests::pagination::PaginatedResponse;
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_mixnet_contract_common::NodeId;
use nym_topology::CachedEpochRewardedSet;
use std::collections::HashMap;
use tracing::{debug, warn};
use utoipa::ToSchema;

pub type PaginatedSemiSkimmedNodes =
    AxumResult<FormattedResponse<PaginatedCachedNodesResponseV2<SemiSkimmedNodeV4>>>;

fn build_response<'a>(
    rewarded_set: &CachedEpochRewardedSet,
    nym_nodes: impl Iterator<Item = &'a NymNodeDescriptionV3>,
    annotations: &HashMap<NodeId, NodeAnnotationV2>,
    current_key_rotation: u32,
) -> Vec<SemiSkimmedNodeV4> {
    let mut nodes = Vec::new();
    let mut skipped = 0;
    for nym_node in nym_nodes {
        let node_id = nym_node.node_id;

        let role: NodeRole = rewarded_set.role(node_id).into();

        // honestly, not sure under what exact circumstances this value could be missing,
        // but in that case just use 0 performance
        let annotation: NodeAnnotationV1 = annotations
            .get(&node_id)
            .cloned()
            .unwrap_or_default()
            .into();

        let Some(node) = nym_node.to_semi_skimmed_node_v4(
            current_key_rotation,
            role,
            annotation.last_24h_performance,
        ) else {
            // without LP details a node can be neither routed to nor dialled over LP
            debug!("node {node_id} published no LP details, skipping");
            skipped += 1;
            continue;
        };

        nodes.push(node);
    }

    if skipped > 0 {
        warn!(
            "{skipped} nodes were left out of the expanded response: they published no LP details"
        );
    }

    nodes
}

#[allow(dead_code)] // not dead, used in OpenAPI docs
#[derive(ToSchema)]
#[schema(title = "PaginatedCachedNodesExpandedV4ResponseSchema")]
pub struct PaginatedCachedNodesExpandedV4ResponseSchema {
    pub refreshed_at: OffsetDateTimeJsonSchemaWrapper,
    #[schema(value_type = SemiSkimmedNodeV4)]
    pub nodes: PaginatedResponse<SemiSkimmedNodeV4>,
}

/// Return all Nym Nodes that are currently bonded.
#[utoipa::path(
    operation_id = "v4_nodes_expanded",
    tag = "Unstable Nym Nodes v4",
    get,
    params(OutputParams),
    path = "/semi-skimmed",
    context_path = "/v4/unstable/nym-nodes",
    responses(
        (status = 200, content(
            (PaginatedCachedNodesExpandedV4ResponseSchema = "application/json"),
            (PaginatedCachedNodesExpandedV4ResponseSchema = "application/yaml"),
            (PaginatedCachedNodesExpandedV4ResponseSchema = "application/bincode")
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
    let status_cache = &state.node_status_cache();
    let annotations = status_cache.node_annotations().await?;

    let contract_cache = state.nym_contract_cache();
    let current_key_rotation = contract_cache.current_key_rotation_id().await?;
    let interval = contract_cache.current_interval().await?;

    let nodes = build_response(
        &rewarded_set,
        all_nym_nodes,
        &annotations,
        current_key_rotation,
    );

    // min of all caches
    let refreshed_at = refreshed_at([
        rewarded_set.timestamp(),
        status_cache.cache_timestamp().await,
        describe_cache.timestamp(),
    ]);

    Ok(output.to_response(PaginatedCachedNodesResponseV2::new_full(
        interval.current_epoch_absolute_id(),
        current_key_rotation,
        refreshed_at,
        nodes,
    )))
}
