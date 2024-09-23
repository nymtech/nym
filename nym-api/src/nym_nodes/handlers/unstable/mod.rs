// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//!   All routes/nodes are split into three tiers:
//!
//!   `/skimmed`
//!     - used by clients
//!     - returns the very basic information for routing purposes
//!
//!   `/semi-skimmed`
//!     - used by other nodes/VPN
//!     - returns more additional information such noise keys
//!
//!   `/full-fat`
//!     - used by explorers, et al.
//!     - returns almost everything there is about the nodes
//!
//!   There's also additional split based on the role:
//!   - `?role` => filters based on the specific role (mixnode/gateway/(in the future: entry/exit))
//!   - `/mixnodes/<tier>` => only returns mixnode role data
//!   - `/gateway/<tier>` => only returns (entry) gateway role data

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::state::AppState;
use axum::extract::Query;
use axum::extract::State;
use axum::{Json, Router};
use nym_api_requests::nym_nodes::{
    CachedNodesResponse, FullFatNode, NodeRole, NodeRoleQueryParam, SemiSkimmedNode, SkimmedNode,
};
use nym_bin_common::version_checker;
use serde::Deserialize;
use std::collections::HashSet;

mod full_fat;
mod semi_skimmed;
mod skimmed;

pub(crate) fn nym_node_routes_unstable() -> axum::Router<AppState> {
    Router::new()
        .route("/skimmed", axum::routing::get(nodes_basic))
        .route("/semi-skimmed", axum::routing::get(nodes_expanded))
        .route("/full-fat", axum::routing::get(nodes_detailed))
        .nest(
            "/gateways",
            Router::new()
                .route("/skimmed", axum::routing::get(gateways_basic))
                .route("/semi-skimmed", axum::routing::get(gateways_expanded))
                .route("/full-fat", axum::routing::get(gateways_detailed)),
        )
        .nest(
            "/mixnodes",
            Router::new()
                .route("/skimmed", axum::routing::get(mixnodes_basic))
                .route("/semi-skimmed", axum::routing::get(mixnodes_expanded))
                .route("/full-fat", axum::routing::get(mixnodes_detailed)),
        )
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
struct NodesParams {
    #[param(inline)]
    role: Option<NodeRoleQueryParam>,
    semver_compatibility: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
struct SemverCompatibilityQueryParam {
    semver_compatibility: Option<String>,
}

impl SemverCompatibilityQueryParam {
    pub fn new(semver_compatibility: Option<String>) -> Self {
        Self {
            semver_compatibility,
        }
    }
}

// async fn nodes_noise() -> AxumResult<Json()> {
//     todo!()
// }

#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParams),
    path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, body = CachedNodesResponse<SkimmedNode>)
    )
)]
async fn nodes_basic(
    state: State<AppState>,
    Query(NodesParams {
        role,
        semver_compatibility,
    }): Query<NodesParams>,
) -> AxumResult<Json<CachedNodesResponse<SkimmedNode>>> {
    if let Some(role) = role {
        match role {
            NodeRoleQueryParam::ActiveMixnode => {
                return mixnodes_basic(
                    state,
                    Query(SemverCompatibilityQueryParam::new(semver_compatibility)),
                )
                .await
            }
            NodeRoleQueryParam::EntryGateway => {
                return gateways_basic(
                    state,
                    Query(SemverCompatibilityQueryParam::new(semver_compatibility)),
                )
                .await;
            }
            _ => {}
        }
    }

    // TODO:
    /*
    - `/v1/unstable/nym-nodes/skimmed` - now works with `exit` parameter
    - `/v1/unstable/nym-nodes/skimmed` - introduced `no-legacy` flag to ignore legacy mixnodes/gateways (where applicable)
    - `/v1/unstable/nym-nodes/skimmed` - will now return **ALL** nodes if no query parameter is provided

     */

    Err(AxumErrorResponse::not_implemented())
}

#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParams),
    path = "/v1/unstable/nym-nodes/semi-skimmed",
    responses(
        (status = 200, body = CachedNodesResponse<SemiSkimmedNode>)
    )
)]
async fn nodes_expanded(
    state: State<AppState>,
    Query(NodesParams {
        role,
        semver_compatibility,
    }): Query<NodesParams>,
) -> AxumResult<Json<CachedNodesResponse<SemiSkimmedNode>>> {
    if let Some(role) = role {
        match role {
            NodeRoleQueryParam::ActiveMixnode => {
                return mixnodes_expanded(
                    state,
                    Query(SemverCompatibilityQueryParam::new(semver_compatibility)),
                )
                .await
            }
            NodeRoleQueryParam::EntryGateway => {
                return gateways_expanded(
                    state,
                    Query(SemverCompatibilityQueryParam::new(semver_compatibility)),
                )
                .await
            }
            _ => {}
        }
    }

    Err(AxumErrorResponse::not_implemented())
}

#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParams),
    path = "/v1/unstable/nym-nodes/full-fat",
    responses(
        (status = 200, body = CachedNodesResponse<FullFatNode>)
    )
)]
async fn nodes_detailed(
    state: State<AppState>,
    Query(NodesParams {
        role,
        semver_compatibility,
    }): Query<NodesParams>,
) -> AxumResult<Json<CachedNodesResponse<FullFatNode>>> {
    if let Some(role) = role {
        match role {
            NodeRoleQueryParam::ActiveMixnode => {
                return mixnodes_detailed(
                    state,
                    Query(SemverCompatibilityQueryParam::new(semver_compatibility)),
                )
                .await
            }
            NodeRoleQueryParam::EntryGateway => {
                return gateways_detailed(
                    state,
                    Query(SemverCompatibilityQueryParam::new(semver_compatibility)),
                )
                .await
            }
            _ => {}
        }
    }

    Err(AxumErrorResponse::not_implemented())
}

#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(SemverCompatibilityQueryParam),
    path = "/v1/unstable/nym-nodes/gateways/skimmed",
    responses(
        (status = 200, body = CachedNodesResponse<SkimmedNode>)
    )
)]
#[deprecated(note = "use '/v1/unstable/nym-nodes/entry-gateways/skimmed/all' instead")]
async fn gateways_basic(
    state: State<AppState>,
    Query(SemverCompatibilityQueryParam {
        semver_compatibility,
    }): Query<SemverCompatibilityQueryParam>,
) -> AxumResult<Json<CachedNodesResponse<SkimmedNode>>> {
    let status_cache = state.node_status_cache();
    let contract_cache = state.nym_contract_cache();
    let describe_cache = state.described_nodes_cache();

    // 1. get the rewarded set
    let rewarded_set = contract_cache
        .rewarded_set()
        .await
        .ok_or_else(AxumErrorResponse::internal)?;

    // determine which gateways are active, i.e. which gateways the clients should be using for connecting and routing the traffic
    let active_gateways = rewarded_set.gateways().into_iter().collect::<HashSet<_>>();

    // 2. grab all annotations so that we could attach scores to the [nym] nodes
    let annotations = status_cache
        .node_annotations()
        .await
        .ok_or_else(AxumErrorResponse::internal)?;

    // 3. grab all legacy gateways
    // due to legacy endpoints we already have fully annotated data on them
    let annotated_legacy_gateways = status_cache
        .annotated_legacy_gateways()
        .await
        .ok_or_else(AxumErrorResponse::internal)?;

    // 4. grab all relevant described nym-nodes
    let describe_cache = describe_cache.get().await?;
    let gateway_capable_nym_nodes = describe_cache.gateway_capable_nym_nodes();

    // 5. only return nodes that are present in the active set
    let mut active_skimmed_gateways = Vec::new();

    for (node_id, legacy) in annotated_legacy_gateways.iter() {
        if !active_gateways.contains(node_id) {
            continue;
        }

        if let Some(semver_compat) = semver_compatibility.as_ref() {
            let version = legacy.version();
            if !version_checker::is_minor_version_compatible(version, semver_compat) {
                continue;
            }
        }

        // if we have self-described info, prefer it over contract data
        if let Some(described) = describe_cache.get_node(node_id) {
            active_skimmed_gateways.push(
                described.to_skimmed_node(NodeRole::EntryGateway, legacy.node_performance.last_24h),
            )
        } else {
            active_skimmed_gateways.push(legacy.into());
        }
    }

    for nym_node in gateway_capable_nym_nodes {
        // if this node is not an active gateway, ignore it
        if !active_gateways.contains(&nym_node.node_id) {
            continue;
        }

        // if we have wrong version, ignore
        if let Some(semver_compat) = semver_compatibility.as_ref() {
            let version = nym_node.version();
            if !version_checker::is_minor_version_compatible(version, semver_compat) {
                continue;
            }
        }

        // NOTE: if we determined our node IS an active gateway, it MUST be EITHER entry or exit
        let role = if rewarded_set.is_exit(&nym_node.node_id) {
            NodeRole::ExitGateway
        } else {
            NodeRole::EntryGateway
        };

        // honestly, not sure under what exact circumstances this value could be missing,
        // but in that case just use 0 performance
        let annotation = annotations
            .get(&nym_node.node_id)
            .copied()
            .unwrap_or_default();

        active_skimmed_gateways
            .push(nym_node.to_skimmed_node(role, annotation.last_24h_performance));
    }

    // min of all caches
    let refreshed_at = [
        rewarded_set.timestamp(),
        annotations.timestamp(),
        describe_cache.timestamp(),
        annotated_legacy_gateways.timestamp(),
    ]
    .into_iter()
    .min()
    .unwrap()
    .into();

    Ok(Json(CachedNodesResponse {
        refreshed_at,
        nodes: active_skimmed_gateways,
    }))
}

#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(SemverCompatibilityQueryParam),
    path = "/v1/unstable/nym-nodes/gateways/semi-skimmed",
    responses(
        (status = 200, body = CachedNodesResponse<SemiSkimmedNode>)
    )
)]
async fn gateways_expanded(
    State(_state): State<AppState>,
    Query(SemverCompatibilityQueryParam {
        semver_compatibility: _semver_compatibility,
    }): Query<SemverCompatibilityQueryParam>,
) -> AxumResult<Json<CachedNodesResponse<SemiSkimmedNode>>> {
    Err(AxumErrorResponse::not_implemented())
}

#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(SemverCompatibilityQueryParam),
    path = "/v1/unstable/nym-nodes/gateways/full-fat",
    responses(
        (status = 200, body = CachedNodesResponse<FullFatNode>)
    )
)]
async fn gateways_detailed(
    State(_state): State<AppState>,
    Query(SemverCompatibilityQueryParam {
        semver_compatibility: _semver_compatibility,
    }): Query<SemverCompatibilityQueryParam>,
) -> AxumResult<Json<CachedNodesResponse<FullFatNode>>> {
    Err(AxumErrorResponse::not_implemented())
}

#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(SemverCompatibilityQueryParam),
    path = "/v1/unstable/nym-nodes/mixnodes/skimmed",
    responses(
        (status = 200, body = CachedNodesResponse<SkimmedNode>)
    )
)]
#[deprecated(note = "use '/v1/unstable/nym-nodes/mixnodes/skimmed/active' instead")]
async fn mixnodes_basic(
    state: State<AppState>,
    Query(SemverCompatibilityQueryParam {
        semver_compatibility,
    }): Query<SemverCompatibilityQueryParam>,
) -> AxumResult<Json<CachedNodesResponse<SkimmedNode>>> {
    let status_cache = state.node_status_cache();
    let contract_cache = state.nym_contract_cache();
    let describe_cache = state.described_nodes_cache();

    // 1. get the rewarded set
    let rewarded_set = contract_cache
        .rewarded_set()
        .await
        .ok_or_else(AxumErrorResponse::internal)?;

    // determine which mixnodes are active, i.e. which mixnodes the clients should be using for routing the traffic
    let active_mixnodes = rewarded_set
        .active_mixnodes()
        .into_iter()
        .collect::<HashSet<_>>();

    // 2. grab all annotations so that we could attach scores to the [nym] nodes
    let annotations = status_cache
        .node_annotations()
        .await
        .ok_or_else(AxumErrorResponse::internal)?;

    // 3. grab all legacy mixnodes
    // due to legacy endpoints we already have fully annotated data on them
    let annotated_legacy_mixnodes = status_cache
        .annotated_legacy_mixnodes()
        .await
        .ok_or_else(AxumErrorResponse::internal)?;

    // 4. grab all relevant described nym-nodes
    let describe_cache = describe_cache.get().await?;
    let mixing_nym_nodes = describe_cache.mixing_nym_nodes();

    // TODO: in the future, only use the self-described cache and simply reject mixnodes that did not expose it

    // 5. only return nodes that are present in the active set
    let mut active_skimmed_mixnodes = Vec::new();

    for (mix_id, legacy) in annotated_legacy_mixnodes.iter() {
        if !active_mixnodes.contains(mix_id) {
            continue;
        }

        if let Some(semver_compat) = semver_compatibility.as_ref() {
            let version = legacy.version();
            if !version_checker::is_minor_version_compatible(version, semver_compat) {
                continue;
            }
        }

        // if we have self-described info, prefer it over contract data
        if let Some(described) = describe_cache.get_node(mix_id) {
            active_skimmed_mixnodes.push(described.to_skimmed_node(
                NodeRole::Mixnode {
                    layer: legacy.mixnode_details.bond_information.layer.into(),
                },
                legacy.node_performance.last_24h,
            ))
        } else {
            active_skimmed_mixnodes.push(legacy.into());
        }
    }

    for nym_node in mixing_nym_nodes {
        // if this node is not an active mixnode, ignore it
        if !active_mixnodes.contains(&nym_node.node_id) {
            continue;
        }

        // if we have wrong version, ignore
        if let Some(semver_compat) = semver_compatibility.as_ref() {
            let version = nym_node.version();
            if !version_checker::is_minor_version_compatible(version, semver_compat) {
                continue;
            }
        }

        // SAFETY: if we determined our node IS active, it MUST have a layer
        // no other thread could have updated the rewarded set as we're still holding the [read] lock on the data
        #[allow(clippy::unwrap_used)]
        let layer = rewarded_set.try_get_mix_layer(&nym_node.node_id).unwrap();

        // honestly, not sure under what exact circumstances this value could be missing,
        // but in that case just use 0 performance
        let annotation = annotations
            .get(&nym_node.node_id)
            .copied()
            .unwrap_or_default();

        active_skimmed_mixnodes.push(
            nym_node.to_skimmed_node(NodeRole::Mixnode { layer }, annotation.last_24h_performance),
        );
    }

    // min of all caches
    let refreshed_at = [
        rewarded_set.timestamp(),
        annotations.timestamp(),
        describe_cache.timestamp(),
        annotated_legacy_mixnodes.timestamp(),
    ]
    .into_iter()
    .min()
    .unwrap()
    .into();

    Ok(Json(CachedNodesResponse {
        refreshed_at,
        nodes: active_skimmed_mixnodes,
    }))
}

#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(SemverCompatibilityQueryParam),
    path = "/v1/unstable/nym-nodes/mixnodes/semi-skimmed",
    responses(
        (status = 200, body = CachedNodesResponse<SemiSkimmedNode>)
    )
)]
async fn mixnodes_expanded(
    State(_state): State<AppState>,
    Query(SemverCompatibilityQueryParam {
        semver_compatibility: _semver_compatibility,
    }): Query<SemverCompatibilityQueryParam>,
) -> AxumResult<Json<CachedNodesResponse<SemiSkimmedNode>>> {
    Err(AxumErrorResponse::not_implemented())
}

#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(SemverCompatibilityQueryParam),
    path = "/v1/unstable/nym-nodes/mixnodes/full-fat",
    responses(
        (status = 200, body = CachedNodesResponse<FullFatNode>)
    )
)]
async fn mixnodes_detailed(
    State(_state): State<AppState>,
    Query(SemverCompatibilityQueryParam {
        semver_compatibility: _semver_compatibility,
    }): Query<SemverCompatibilityQueryParam>,
) -> AxumResult<Json<CachedNodesResponse<FullFatNode>>> {
    Err(AxumErrorResponse::not_implemented())
}
