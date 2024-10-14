// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_describe_cache::DescribedNodes;
use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::nym_contract_cache::cache::CachedRewardedSet;
use crate::nym_nodes::handlers::unstable::helpers::{refreshed_at, semver, LegacyAnnotation};
use crate::nym_nodes::handlers::unstable::{NodesParams, NodesParamsWithRole};
use crate::support::caching::Cache;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use nym_api_requests::models::{NodeAnnotation, NymNodeDescription};
use nym_api_requests::nym_nodes::{
    CachedNodesResponse, NodeRole, NodeRoleQueryParam, PaginatedCachedNodesResponse, SkimmedNode,
};
use nym_mixnet_contract_common::NodeId;
use std::collections::HashMap;
use std::future::Future;
use tokio::sync::RwLockReadGuard;
use tracing::trace;

pub type PaginatedSkimmedNodes = AxumResult<Json<PaginatedCachedNodesResponse<SkimmedNode>>>;

/// Given all relevant caches, build part of response for JUST Nym Nodes
fn build_nym_nodes_response<'a, NI>(
    rewarded_set: &CachedRewardedSet,
    required_semver: &Option<String>,
    nym_nodes_subset: NI,
    annotations: &HashMap<NodeId, NodeAnnotation>,
    active_only: bool,
) -> Vec<SkimmedNode>
where
    NI: Iterator<Item = &'a NymNodeDescription> + 'a,
{
    let mut nodes = Vec::new();
    for nym_node in nym_nodes_subset {
        let node_id = nym_node.node_id;

        // if we have wrong version, ignore
        if !semver(required_semver, nym_node.version()) {
            continue;
        }

        let role: NodeRole = rewarded_set.role(node_id).into();

        // if the role is inactive, see if our filter allows it
        if active_only && role.is_inactive() {
            continue;
        }

        // honestly, not sure under what exact circumstances this value could be missing,
        // but in that case just use 0 performance
        let annotation = annotations.get(&node_id).copied().unwrap_or_default();

        nodes.push(nym_node.to_skimmed_node(role, annotation.last_24h_performance));
    }
    nodes
}

/// Given all relevant caches, add appropriate legacy nodes to the part of the response
fn add_legacy<LN>(
    nodes: &mut Vec<SkimmedNode>,
    required_semver: &Option<String>,
    rewarded_set: &CachedRewardedSet,
    describe_cache: &DescribedNodes,
    annotated_legacy_nodes: &HashMap<NodeId, LN>,
    active_only: bool,
) where
    LN: LegacyAnnotation,
{
    for (node_id, legacy) in annotated_legacy_nodes.iter() {
        // if we have wrong version, ignore
        if !semver(required_semver, legacy.version()) {
            continue;
        }

        let role: NodeRole = rewarded_set.role(*node_id).into();

        // if the role is inactive, see if our filter allows it
        if active_only && role.is_inactive() {
            continue;
        }

        // if we have self-described info, prefer it over contract data
        if let Some(described) = describe_cache.get_node(node_id) {
            nodes.push(described.to_skimmed_node(role, legacy.performance()))
        } else {
            match legacy.try_to_skimmed_node(role) {
                Ok(node) => nodes.push(node),
                Err(err) => {
                    let id = legacy.identity();
                    trace!("node {id} is malformed: {err}")
                }
            }
        }
    }
}

// hehe, what an abomination, but it's used in multiple different places and I hate copy-pasting code,
// especially if it has multiple loops, etc
async fn build_skimmed_nodes_response<'a, NI, LG, Fut, LN>(
    state: &'a AppState,
    Query(query_params): Query<NodesParams>,
    nym_nodes_subset: NI,
    annotated_legacy_nodes_getter: LG,
    active_only: bool,
) -> PaginatedSkimmedNodes
where
    // iterator returning relevant subset of nym-nodes (like mixing nym-nodes, entries, etc.)
    NI: Iterator<Item = &'a NymNodeDescription> + 'a,

    // async function that returns cache of appropriate legacy nodes (mixnodes or gateways)
    LG: Fn(&'a AppState) -> Fut,
    Fut:
        Future<Output = Result<RwLockReadGuard<'a, Cache<HashMap<NodeId, LN>>>, AxumErrorResponse>>,

    // the legacy node (MixNodeBondAnnotated or GatewayBondAnnotated)
    LN: LegacyAnnotation + 'a,
{
    // TODO: implement it
    let _ = query_params.per_page;
    let _ = query_params.page;
    let semver_compatibility = query_params.semver_compatibility;

    // 1. get the rewarded set
    let rewarded_set = state.rewarded_set().await?;

    // 2. grab all annotations so that we could attach scores to the [nym] nodes
    let annotations = state.node_annotations().await?;

    // 3. implicitly grab the relevant described nodes
    // (ideally it'd be tied directly to the NI iterator, but I couldn't defeat the compiler)
    let describe_cache = state.describe_nodes_cache_data().await?;

    // 4. start building the response
    let mut nodes = build_nym_nodes_response(
        &rewarded_set,
        &semver_compatibility,
        nym_nodes_subset,
        &annotations,
        active_only,
    );

    // 5. if we allow legacy nodes, repeat the procedure for them, otherwise return just nym-nodes
    if let Some(true) = query_params.no_legacy {
        // min of all caches
        let refreshed_at = refreshed_at([
            rewarded_set.timestamp(),
            annotations.timestamp(),
            describe_cache.timestamp(),
        ]);

        return Ok(Json(PaginatedCachedNodesResponse::new_full(
            refreshed_at,
            nodes,
        )));
    }

    // 6. grab relevant legacy nodes
    // (due to the existence of the legacy endpoints, we already have fully annotated data on them)
    let annotated_legacy_nodes = annotated_legacy_nodes_getter(state).await?;
    add_legacy(
        &mut nodes,
        &semver_compatibility,
        &rewarded_set,
        &describe_cache,
        &annotated_legacy_nodes,
        active_only,
    );

    // min of all caches
    let refreshed_at = refreshed_at([
        rewarded_set.timestamp(),
        annotations.timestamp(),
        describe_cache.timestamp(),
        annotated_legacy_nodes.timestamp(),
    ]);

    Ok(Json(PaginatedCachedNodesResponse::new_full(
        refreshed_at,
        nodes,
    )))
}

/// Deprecated query that gets ALL gateways
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParams),
    path = "/gateways/skimmed",
    context_path = "/v1/unstable/nym-nodes",
    responses(
        (status = 200, body = CachedNodesResponse<SkimmedNode>)
    )
)]
#[deprecated(note = "use '/v1/unstable/nym-nodes/entry-gateways/skimmed/all' instead")]
pub(super) async fn deprecated_gateways_basic(
    state: State<AppState>,
    query_params: Query<NodesParams>,
) -> AxumResult<Json<CachedNodesResponse<SkimmedNode>>> {
    // 1. call '/v1/unstable/skimmed/entry-gateways/all'
    let all_gateways = entry_gateways_basic_all(state, query_params).await?;

    // 3. return result
    Ok(Json(CachedNodesResponse {
        refreshed_at: all_gateways.refreshed_at,
        // 2. remove pagination
        nodes: all_gateways.0.nodes.data,
    }))
}

/// Deprecated query that gets ACTIVE-ONLY mixnodes
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParams),
    path = "/mixnodes/skimmed",
    context_path = "/v1/unstable/nym-nodes",
    responses(
        (status = 200, body = CachedNodesResponse<SkimmedNode>)
    )
)]
#[deprecated(note = "use '/v1/unstable/nym-nodes/mixnodes/skimmed/active' instead")]
pub(super) async fn deprecated_mixnodes_basic(
    state: State<AppState>,
    query_params: Query<NodesParams>,
) -> AxumResult<Json<CachedNodesResponse<SkimmedNode>>> {
    // 1. call '/v1/unstable/nym-nodes/skimmed/mixnodes/active'
    let active_mixnodes = mixnodes_basic_active(state, query_params).await?;

    // 3. return result
    Ok(Json(CachedNodesResponse {
        refreshed_at: active_mixnodes.refreshed_at,
        // 2. remove pagination
        nodes: active_mixnodes.0.nodes.data,
    }))
}

async fn nodes_basic(
    state: State<AppState>,
    Query(query_params): Query<NodesParams>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    // unfortunately we have to build the response semi-manually here as we need to add two sources of legacy nodes

    // 1. grab all relevant described nym-nodes
    let rewarded_set = state.rewarded_set().await?;
    let semver_compatibility = &query_params.semver_compatibility;

    let describe_cache = state.describe_nodes_cache_data().await?;
    let all_nym_nodes = describe_cache.all_nym_nodes();
    let annotations = state.node_annotations().await?;
    let legacy_mixnodes = state.legacy_mixnode_annotations().await?;
    let legacy_gateways = state.legacy_gateways_annotations().await?;

    let mut nodes = build_nym_nodes_response(
        &rewarded_set,
        semver_compatibility,
        all_nym_nodes,
        &annotations,
        active_only,
    );

    // add legacy gateways to the response
    add_legacy(
        &mut nodes,
        semver_compatibility,
        &rewarded_set,
        &describe_cache,
        &legacy_gateways,
        active_only,
    );

    // add legacy mixnodes to the response
    add_legacy(
        &mut nodes,
        semver_compatibility,
        &rewarded_set,
        &describe_cache,
        &legacy_mixnodes,
        active_only,
    );

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

/// Return all Nym Nodes and optionally legacy mixnodes/gateways (if `no-legacy` flag is not used)
/// that are currently bonded.
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParamsWithRole),
    path = "/",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, body = PaginatedCachedNodesResponse<SkimmedNode>)
    )
)]
pub(super) async fn nodes_basic_all(
    state: State<AppState>,
    Query(query_params): Query<NodesParamsWithRole>,
) -> PaginatedSkimmedNodes {
    if let Some(role) = query_params.role {
        return match role {
            NodeRoleQueryParam::ActiveMixnode => {
                mixnodes_basic_all(state, Query(query_params.into())).await
            }
            NodeRoleQueryParam::EntryGateway => {
                entry_gateways_basic_all(state, Query(query_params.into())).await
            }
            NodeRoleQueryParam::ExitGateway => {
                exit_gateways_basic_all(state, Query(query_params.into())).await
            }
        };
    }

    nodes_basic(state, Query(query_params.into()), false).await
}

/// Return Nym Nodes and optionally legacy mixnodes/gateways (if `no-legacy` flag is not used)
/// that are currently bonded and are in the **active set**
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParamsWithRole),
    path = "/active",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, body = PaginatedCachedNodesResponse<SkimmedNode>)
    )
)]
pub(super) async fn nodes_basic_active(
    state: State<AppState>,
    Query(query_params): Query<NodesParamsWithRole>,
) -> PaginatedSkimmedNodes {
    if let Some(role) = query_params.role {
        return match role {
            NodeRoleQueryParam::ActiveMixnode => {
                mixnodes_basic_active(state, Query(query_params.into())).await
            }
            NodeRoleQueryParam::EntryGateway => {
                entry_gateways_basic_active(state, Query(query_params.into())).await
            }
            NodeRoleQueryParam::ExitGateway => {
                exit_gateways_basic_active(state, Query(query_params.into())).await
            }
        };
    }

    nodes_basic(state, Query(query_params.into()), true).await
}

async fn mixnodes_basic(
    state: State<AppState>,
    query_params: Query<NodesParams>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    // 1. grab all relevant described nym-nodes
    let describe_cache = state.describe_nodes_cache_data().await?;
    let mixing_nym_nodes = describe_cache.mixing_nym_nodes();

    build_skimmed_nodes_response(
        &state.0,
        query_params,
        mixing_nym_nodes,
        |state| state.legacy_mixnode_annotations(),
        active_only,
    )
    .await
}

/// Returns Nym Nodes and optionally legacy mixnodes (if `no-legacy` flag is not used)
/// that are currently bonded and support mixing role.
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParamsWithRole),
    path = "/mixnodes/all",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, body = PaginatedCachedNodesResponse<SkimmedNode>)
    )
)]
pub(super) async fn mixnodes_basic_all(
    state: State<AppState>,
    query_params: Query<NodesParams>,
) -> PaginatedSkimmedNodes {
    mixnodes_basic(state, query_params, false).await
}

/// Returns Nym Nodes and optionally legacy mixnodes (if `no-legacy` flag is not used)
/// that are currently bonded and are in the active set with one of the mixing roles.
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParamsWithRole),
    path = "/mixnodes/active",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, body = PaginatedCachedNodesResponse<SkimmedNode>)
    )
)]
pub(super) async fn mixnodes_basic_active(
    state: State<AppState>,
    query_params: Query<NodesParams>,
) -> PaginatedSkimmedNodes {
    mixnodes_basic(state, query_params, true).await
}

async fn entry_gateways_basic(
    state: State<AppState>,
    query_params: Query<NodesParams>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    // 1. grab all relevant described nym-nodes
    let describe_cache = state.describe_nodes_cache_data().await?;
    let mixing_nym_nodes = describe_cache.entry_capable_nym_nodes();

    build_skimmed_nodes_response(
        &state.0,
        query_params,
        mixing_nym_nodes,
        |state| state.legacy_gateways_annotations(),
        active_only,
    )
    .await
}

/// Returns Nym Nodes and optionally legacy gateways (if `no-legacy` flag is not used)
/// that are currently bonded and are in the active set with the entry role.
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParamsWithRole),
    path = "/entry-gateways/active",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, body = PaginatedCachedNodesResponse<SkimmedNode>)
    )
)]
pub(super) async fn entry_gateways_basic_active(
    state: State<AppState>,
    query_params: Query<NodesParams>,
) -> PaginatedSkimmedNodes {
    entry_gateways_basic(state, query_params, true).await
}

/// Returns Nym Nodes and optionally legacy gateways (if `no-legacy` flag is not used)
/// that are currently bonded and support entry gateway role.
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParamsWithRole),
    path = "/entry-gateways/all",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, body = PaginatedCachedNodesResponse<SkimmedNode>)
    )
)]
pub(super) async fn entry_gateways_basic_all(
    state: State<AppState>,
    query_params: Query<NodesParams>,
) -> PaginatedSkimmedNodes {
    entry_gateways_basic(state, query_params, false).await
}

async fn exit_gateways_basic(
    state: State<AppState>,
    query_params: Query<NodesParams>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    // 1. grab all relevant described nym-nodes
    let describe_cache = state.describe_nodes_cache_data().await?;
    let mixing_nym_nodes = describe_cache.exit_capable_nym_nodes();

    build_skimmed_nodes_response(
        &state.0,
        query_params,
        mixing_nym_nodes,
        |state| state.legacy_gateways_annotations(),
        active_only,
    )
    .await
}

/// Returns Nym Nodes and optionally legacy gateways (if `no-legacy` flag is not used)
/// that are currently bonded and are in the active set with the exit role.
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParamsWithRole),
    path = "/exit-gateways/active",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, body = PaginatedCachedNodesResponse<SkimmedNode>)
    )
)]
pub(super) async fn exit_gateways_basic_active(
    state: State<AppState>,
    query_params: Query<NodesParams>,
) -> PaginatedSkimmedNodes {
    exit_gateways_basic(state, query_params, true).await
}

/// Returns Nym Nodes and optionally legacy gateways (if `no-legacy` flag is not used)
/// that are currently bonded and support exit gateway role.
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParamsWithRole),
    path = "/exit-gateways/all",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, body = PaginatedCachedNodesResponse<SkimmedNode>)
    )
)]
pub(super) async fn exit_gateways_basic_all(
    state: State<AppState>,
    query_params: Query<NodesParams>,
) -> PaginatedSkimmedNodes {
    exit_gateways_basic(state, query_params, false).await
}
