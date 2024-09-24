// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::nym_nodes::handlers::unstable::{NodesParams, NodesParamsWithRole};
use crate::support::http::helpers::PaginationRequest;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use nym_api_requests::nym_nodes::{
    CachedNodesResponse, NodeRole, NodeRoleQueryParam, PaginatedCachedNodesResponse, SkimmedNode,
};
use nym_bin_common::version_checker;
use std::collections::HashSet;

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
    // TODO:
    // 1. call '/v1/unstable/entry-gateways/mixnodes/skimmed/all'
    // 2. remove pagination
    // 3. return result

    todo!()

    // let status_cache = state.node_status_cache();
    // let contract_cache = state.nym_contract_cache();
    // let describe_cache = state.described_nodes_cache();
    //
    // // 1. get the rewarded set
    // let rewarded_set = contract_cache
    //     .rewarded_set()
    //     .await
    //     .ok_or_else(AxumErrorResponse::internal)?;
    //
    // // determine which gateways are active, i.e. which gateways the clients should be using for connecting and routing the traffic
    // let active_gateways = rewarded_set.gateways().into_iter().collect::<HashSet<_>>();
    //
    // // 2. grab all annotations so that we could attach scores to the [nym] nodes
    // let annotations = status_cache
    //     .node_annotations()
    //     .await
    //     .ok_or_else(AxumErrorResponse::internal)?;
    //
    // // 3. grab all legacy gateways
    // // due to legacy endpoints we already have fully annotated data on them
    // let annotated_legacy_gateways = status_cache
    //     .annotated_legacy_gateways()
    //     .await
    //     .ok_or_else(AxumErrorResponse::internal)?;
    //
    // // 4. grab all relevant described nym-nodes
    // let describe_cache = describe_cache.get().await?;
    // let gateway_capable_nym_nodes = describe_cache.gateway_capable_nym_nodes();
    //
    // // 5. only return nodes that are present in the active set
    // let mut active_skimmed_gateways = Vec::new();
    //
    // for (node_id, legacy) in annotated_legacy_gateways.iter() {
    //     if !active_gateways.contains(node_id) {
    //         continue;
    //     }
    //
    //     if let Some(semver_compat) = semver_compatibility.as_ref() {
    //         let version = legacy.version();
    //         if !version_checker::is_minor_version_compatible(version, semver_compat) {
    //             continue;
    //         }
    //     }
    //
    //     // if we have self-described info, prefer it over contract data
    //     if let Some(described) = describe_cache.get_node(node_id) {
    //         active_skimmed_gateways.push(
    //             described.to_skimmed_node(NodeRole::EntryGateway, legacy.node_performance.last_24h),
    //         )
    //     } else {
    //         active_skimmed_gateways.push(legacy.into());
    //     }
    // }
    //
    // for nym_node in gateway_capable_nym_nodes {
    //     // if this node is not an active gateway, ignore it
    //     if !active_gateways.contains(&nym_node.node_id) {
    //         continue;
    //     }
    //
    //     // if we have wrong version, ignore
    //     if let Some(semver_compat) = semver_compatibility.as_ref() {
    //         let version = nym_node.version();
    //         if !version_checker::is_minor_version_compatible(version, semver_compat) {
    //             continue;
    //         }
    //     }
    //
    //     // NOTE: if we determined our node IS an active gateway, it MUST be EITHER entry or exit
    //     let role = if rewarded_set.is_exit(&nym_node.node_id) {
    //         NodeRole::ExitGateway
    //     } else {
    //         NodeRole::EntryGateway
    //     };
    //
    //     // honestly, not sure under what exact circumstances this value could be missing,
    //     // but in that case just use 0 performance
    //     let annotation = annotations
    //         .get(&nym_node.node_id)
    //         .copied()
    //         .unwrap_or_default();
    //
    //     active_skimmed_gateways
    //         .push(nym_node.to_skimmed_node(role, annotation.last_24h_performance));
    // }
    //
    // // min of all caches
    // let refreshed_at = [
    //     rewarded_set.timestamp(),
    //     annotations.timestamp(),
    //     describe_cache.timestamp(),
    //     annotated_legacy_gateways.timestamp(),
    // ]
    // .into_iter()
    // .min()
    // .unwrap()
    // .into();
    //
    // Ok(Json(CachedNodesResponse {
    //     refreshed_at,
    //     nodes: active_skimmed_gateways,
    // }))
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
    // TODO:
    // 1. call '/v1/unstable/nym-nodes/mixnodes/skimmed/active'
    // 2. remove pagination
    // 3. return result

    todo!()
    //
    // let status_cache = state.node_status_cache();
    // let contract_cache = state.nym_contract_cache();
    // let describe_cache = state.described_nodes_cache();
    //
    // // 1. get the rewarded set
    // let rewarded_set = contract_cache
    //     .rewarded_set()
    //     .await
    //     .ok_or_else(AxumErrorResponse::internal)?;
    //
    // // determine which mixnodes are active, i.e. which mixnodes the clients should be using for routing the traffic
    // let active_mixnodes = rewarded_set
    //     .active_mixnodes()
    //     .into_iter()
    //     .collect::<HashSet<_>>();
    //
    // // 2. grab all annotations so that we could attach scores to the [nym] nodes
    // let annotations = status_cache
    //     .node_annotations()
    //     .await
    //     .ok_or_else(AxumErrorResponse::internal)?;
    //
    // // 3. grab all legacy mixnodes
    // // due to legacy endpoints we already have fully annotated data on them
    // let annotated_legacy_mixnodes = status_cache
    //     .annotated_legacy_mixnodes()
    //     .await
    //     .ok_or_else(AxumErrorResponse::internal)?;
    //
    // // 4. grab all relevant described nym-nodes
    // let describe_cache = describe_cache.get().await?;
    // let mixing_nym_nodes = describe_cache.mixing_nym_nodes();
    //
    // // TODO: in the future, only use the self-described cache and simply reject mixnodes that did not expose it
    //
    // // 5. only return nodes that are present in the active set
    // let mut active_skimmed_mixnodes = Vec::new();
    //
    // for (mix_id, legacy) in annotated_legacy_mixnodes.iter() {
    //     if !active_mixnodes.contains(mix_id) {
    //         continue;
    //     }
    //
    //     if let Some(semver_compat) = semver_compatibility.as_ref() {
    //         let version = legacy.version();
    //         if !version_checker::is_minor_version_compatible(version, semver_compat) {
    //             continue;
    //         }
    //     }
    //
    //     // if we have self-described info, prefer it over contract data
    //     if let Some(described) = describe_cache.get_node(mix_id) {
    //         active_skimmed_mixnodes.push(described.to_skimmed_node(
    //             NodeRole::Mixnode {
    //                 layer: legacy.mixnode_details.bond_information.layer.into(),
    //             },
    //             legacy.node_performance.last_24h,
    //         ))
    //     } else {
    //         active_skimmed_mixnodes.push(legacy.into());
    //     }
    // }
    //
    // for nym_node in mixing_nym_nodes {
    //     // if this node is not an active mixnode, ignore it
    //     if !active_mixnodes.contains(&nym_node.node_id) {
    //         continue;
    //     }
    //
    //     // if we have wrong version, ignore
    //     if let Some(semver_compat) = semver_compatibility.as_ref() {
    //         let version = nym_node.version();
    //         if !version_checker::is_minor_version_compatible(version, semver_compat) {
    //             continue;
    //         }
    //     }
    //
    //     // SAFETY: if we determined our node IS active, it MUST have a layer
    //     // no other thread could have updated the rewarded set as we're still holding the [read] lock on the data
    //     #[allow(clippy::unwrap_used)]
    //     let layer = rewarded_set.try_get_mix_layer(&nym_node.node_id).unwrap();
    //
    //     // honestly, not sure under what exact circumstances this value could be missing,
    //     // but in that case just use 0 performance
    //     let annotation = annotations
    //         .get(&nym_node.node_id)
    //         .copied()
    //         .unwrap_or_default();
    //
    //     active_skimmed_mixnodes.push(
    //         nym_node.to_skimmed_node(NodeRole::Mixnode { layer }, annotation.last_24h_performance),
    //     );
    // }
    //
    // // min of all caches
    // let refreshed_at = [
    //     rewarded_set.timestamp(),
    //     annotations.timestamp(),
    //     describe_cache.timestamp(),
    //     annotated_legacy_mixnodes.timestamp(),
    // ]
    // .into_iter()
    // .min()
    // .unwrap()
    // .into();
    //
    // Ok(Json(CachedNodesResponse {
    //     refreshed_at,
    //     nodes: active_skimmed_mixnodes,
    // }))
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
    query_params: Query<NodesParamsWithRole>,
) -> PaginatedSkimmedNodes {
    if let Some(role) = query_params.0.role {
        return match role {
            NodeRoleQueryParam::ActiveMixnode => {
                mixnodes_basic_all(state, Query(query_params.0.into())).await
            }
            NodeRoleQueryParam::EntryGateway => {
                entry_gateways_basic_all(state, Query(query_params.0.into())).await
            }
            NodeRoleQueryParam::ExitGateway => {
                exit_gateways_basic_all(state, Query(query_params.0.into())).await
            }
        };
    }

    let query_params = query_params.0;

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
    todo!()
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
    todo!()
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
    todo!()
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
    todo!()
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
    todo!()
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
    todo!()
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
    todo!()
}
