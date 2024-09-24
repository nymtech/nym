// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::nym_nodes::handlers::unstable::helpers::{refreshed_at, semver};
use crate::nym_nodes::handlers::unstable::{NodesParams, NodesParamsWithRole};
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use nym_api_requests::nym_nodes::{
    CachedNodesResponse, NodeRole, NodeRoleQueryParam, PaginatedCachedNodesResponse, SkimmedNode,
};
use tracing::trace;

pub type PaginatedSkimmedNodes = AxumResult<Json<PaginatedCachedNodesResponse<SkimmedNode>>>;

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
pub(super) async fn nodes_basic(
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

    // TODO: implement pagination

    let semver = query_params.semver_compatibility;
    let no_legacy = query_params.no_legacy;

    // TODO:
    /*
    - `/v1/unstable/nym-nodes/skimmed` - now works with `exit` parameter
    - `/v1/unstable/nym-nodes/skimmed` - introduced `no-legacy` flag to ignore legacy mixnodes/gateways (where applicable)
    - `/v1/unstable/nym-nodes/skimmed` - will now return **ALL** nodes if no query parameter is provided

     */

    Err(AxumErrorResponse::not_implemented())
}

//     - `/v1/unstable/nym-nodes/skimmed/active` - returns all Nym Nodes **AND** legacy mixnodes **AND** legacy gateways that are currently in the active set, unless `no-legacy` parameter is used
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
    query_params: Query<NodesParamsWithRole>,
) -> PaginatedSkimmedNodes {
    // TODO: implement it
    let _ = query_params.per_page;
    let _ = query_params.page;
    todo!()
}

/// Return Nym Nodes and optionally legacy mixnodes/gateways (if `no-legacy` flag is not used)
/// that are currently bonded and are in the **standby set**
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParamsWithRole),
    path = "/skimmed/standby",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, body = PaginatedCachedNodesResponse<SkimmedNode>)
    )
)]
pub(super) async fn nodes_basic_standby(
    state: State<AppState>,
    query_params: Query<NodesParamsWithRole>,
) -> PaginatedSkimmedNodes {
    // TODO: implement it
    let _ = query_params.per_page;
    let _ = query_params.page;
    todo!()
}

async fn mixnodes_basic(
    state: State<AppState>,
    Query(query_params): Query<NodesParams>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    // TODO: implement it
    let _ = query_params.per_page;
    let _ = query_params.page;
    let semver_compatibility = query_params.semver_compatibility;

    // 1. get the rewarded set
    let rewarded_set = state.rewarded_set().await?;

    // 2. grab all annotations so that we could attach scores to the [nym] nodes
    let annotations = state.node_annotations().await?;

    // 3. grab all relevant described nym-nodes
    let describe_cache = state.describe_nodes_cache_data().await?;
    let mixing_nym_nodes = describe_cache.mixing_nym_nodes();

    // 4. start building the response
    let mut nodes = Vec::new();

    for nym_node in mixing_nym_nodes {
        let node_id = nym_node.node_id;

        // if this node is not an active mixnode, ignore it
        if active_only && !rewarded_set.is_active_mixnode(&node_id) {
            continue;
        }

        // if we have wrong version, ignore
        if !semver(&semver_compatibility, nym_node.version()) {
            continue;
        }

        let role = match rewarded_set.try_get_mix_layer(&node_id) {
            Some(layer) => NodeRole::Mixnode { layer },
            None => NodeRole::Inactive,
        };

        // honestly, not sure under what exact circumstances this value could be missing,
        // but in that case just use 0 performance
        let annotation = annotations.get(&node_id).copied().unwrap_or_default();

        nodes.push(nym_node.to_skimmed_node(role, annotation.last_24h_performance));
    }

    // 5. if we allow legacy mixnodes, repeat the procedure for mixnodes, otherwise return just nym-nodes
    if query_params.no_legacy {
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

    // 6. grab all legacy mixnodes
    // due to legacy endpoints we already have fully annotated data on them
    let annotated_legacy_mixnodes = state.legacy_mixnode_annotations().await?;

    for (mix_id, legacy) in annotated_legacy_mixnodes.iter() {
        // if this node is not an active mixnode, ignore it
        if active_only && !rewarded_set.is_active_mixnode(mix_id) {
            continue;
        }

        // if we have wrong version, ignore
        if !semver(&semver_compatibility, legacy.version()) {
            continue;
        }

        let role = match rewarded_set.try_get_mix_layer(mix_id) {
            Some(layer) => NodeRole::Mixnode { layer },
            None => NodeRole::Inactive,
        };

        // if we have self-described info, prefer it over contract data
        if let Some(described) = describe_cache.get_node(mix_id) {
            nodes.push(described.to_skimmed_node(role, legacy.node_performance.last_24h))
        } else {
            match legacy.try_to_skimmed_node(role) {
                Ok(node) => nodes.push(node),
                Err(err) => {
                    let id = legacy.identity_key();
                    trace!("node {id} is malformed: {err}")
                }
            }
        }
    }

    // min of all caches
    let refreshed_at = refreshed_at([
        rewarded_set.timestamp(),
        annotations.timestamp(),
        describe_cache.timestamp(),
        annotated_legacy_mixnodes.timestamp(),
    ]);

    Ok(Json(PaginatedCachedNodesResponse::new_full(
        refreshed_at,
        nodes,
    )))
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
    Query(query_params): Query<NodesParams>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    // TODO: implement it
    let _ = query_params.per_page;
    let _ = query_params.page;
    let semver_compatibility = query_params.semver_compatibility;

    // 1. get the rewarded set
    let rewarded_set = state.rewarded_set().await?;

    // 2. grab all annotations so that we could attach scores to the [nym] nodes
    let annotations = state.node_annotations().await?;

    // 3. grab all relevant described nym-nodes
    let describe_cache = state.describe_nodes_cache_data().await?;
    let gateway_capable_nym_nodes = describe_cache.entry_capable_nym_nodes();

    // 4. start building the response
    let mut nodes = Vec::new();

    for nym_node in gateway_capable_nym_nodes {
        let node_id = nym_node.node_id;

        // if this node is not an active gateway, ignore it
        if active_only && !rewarded_set.is_entry(&node_id) {
            continue;
        }

        // if we have wrong version, ignore
        if !semver(&semver_compatibility, nym_node.version()) {
            continue;
        }

        let role = match rewarded_set.is_entry(&node_id) {
            true => NodeRole::EntryGateway,
            false => NodeRole::Inactive,
        };

        // honestly, not sure under what exact circumstances this value could be missing,
        // but in that case just use 0 performance
        let annotation = annotations.get(&node_id).copied().unwrap_or_default();

        nodes.push(nym_node.to_skimmed_node(role, annotation.last_24h_performance));
    }

    // 5. if we allow legacy gateways, repeat the procedure for gateways, otherwise return just nym-nodes
    if query_params.no_legacy {
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

    // 6. grab all legacy gateways
    // due to legacy endpoints we already have fully annotated data on them
    let annotated_legacy_gateways = state.legacy_gateways_annotations().await?;

    for (node_id, legacy) in annotated_legacy_gateways.iter() {
        // if this node is not an active gateway, ignore it
        if active_only && !rewarded_set.is_entry(&node_id) {
            continue;
        }

        // if we have wrong version, ignore
        if !semver(&semver_compatibility, legacy.version()) {
            continue;
        }

        let role = match rewarded_set.is_entry(&node_id) {
            true => NodeRole::EntryGateway,
            false => NodeRole::Inactive,
        };

        // if we have self-described info, prefer it over contract data
        if let Some(described) = describe_cache.get_node(node_id) {
            nodes.push(described.to_skimmed_node(role, legacy.node_performance.last_24h))
        } else {
            match legacy.try_to_skimmed_node(role) {
                Ok(node) => nodes.push(node),
                Err(err) => {
                    let id = legacy.gateway_bond.identity();
                    trace!("node {id} is malformed: {err}")
                }
            }
        }
    }

    // min of all caches
    let refreshed_at = refreshed_at([
        rewarded_set.timestamp(),
        annotations.timestamp(),
        describe_cache.timestamp(),
        annotated_legacy_gateways.timestamp(),
    ]);

    Ok(Json(PaginatedCachedNodesResponse::new_full(
        refreshed_at,
        nodes,
    )))
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
    Query(query_params): Query<NodesParams>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    // TODO: implement it
    let _ = query_params.per_page;
    let _ = query_params.page;
    let semver_compatibility = query_params.semver_compatibility;

    // 1. get the rewarded set
    let rewarded_set = state.rewarded_set().await?;

    // 2. grab all annotations so that we could attach scores to the [nym] nodes
    let annotations = state.node_annotations().await?;

    // 3. grab all relevant described nym-nodes
    let describe_cache = state.describe_nodes_cache_data().await?;
    let gateway_capable_nym_nodes = describe_cache.exit_capable_nym_nodes();

    // 4. start building the response
    let mut nodes = Vec::new();

    for nym_node in gateway_capable_nym_nodes {
        let node_id = nym_node.node_id;

        // if this node is not an active gateway, ignore it
        if active_only && !rewarded_set.is_exit(&node_id) {
            continue;
        }

        // if we have wrong version, ignore
        if !semver(&semver_compatibility, nym_node.version()) {
            continue;
        }

        let role = match rewarded_set.is_exit(&node_id) {
            true => NodeRole::ExitGateway,
            false => NodeRole::Inactive,
        };

        // honestly, not sure under what exact circumstances this value could be missing,
        // but in that case just use 0 performance
        let annotation = annotations.get(&node_id).copied().unwrap_or_default();

        nodes.push(nym_node.to_skimmed_node(role, annotation.last_24h_performance));
    }

    // 5. if we allow legacy gateways, repeat the procedure for gateways, otherwise return just nym-nodes
    if query_params.no_legacy {
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

    // 6. grab all legacy gateways
    // due to legacy endpoints we already have fully annotated data on them
    let annotated_legacy_gateways = state.legacy_gateways_annotations().await?;

    for (node_id, legacy) in annotated_legacy_gateways.iter() {
        // if this node is not an active gateway, ignore it
        if active_only && !rewarded_set.is_exit(&node_id) {
            continue;
        }

        // if we have wrong version, ignore
        if !semver(&semver_compatibility, legacy.version()) {
            continue;
        }

        let role = match rewarded_set.is_exit(&node_id) {
            true => NodeRole::ExitGateway,
            false => NodeRole::Inactive,
        };

        // if we have self-described info, prefer it over contract data
        if let Some(described) = describe_cache.get_node(node_id) {
            nodes.push(described.to_skimmed_node(role, legacy.node_performance.last_24h))
        } else {
            match legacy.try_to_skimmed_node(role) {
                Ok(node) => nodes.push(node),
                Err(err) => {
                    let id = legacy.gateway_bond.identity();
                    trace!("node {id} is malformed: {err}")
                }
            }
        }
    }

    // min of all caches
    let refreshed_at = refreshed_at([
        rewarded_set.timestamp(),
        annotations.timestamp(),
        describe_cache.timestamp(),
        annotated_legacy_gateways.timestamp(),
    ]);

    Ok(Json(PaginatedCachedNodesResponse::new_full(
        refreshed_at,
        nodes,
    )))
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
