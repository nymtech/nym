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
use crate::v2::AxumAppState;
use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use nym_api_requests::nym_nodes::{
    CachedNodesResponse, FullFatNode, NodeRoleQueryParam, SemiSkimmedNode, SkimmedNode,
};
use nym_bin_common::version_checker;
use serde::Deserialize;
use std::cmp::min;
use std::ops::Deref;

#[derive(Debug, Deserialize)]
pub(super) struct NodesParams {
    role: Option<NodeRoleQueryParam>,
    semver_compatibility: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SemverCompatibilityQueryParam {
    semver_compatibility: Option<String>,
}

impl SemverCompatibilityQueryParam {
    pub fn new(semver_compatibility: Option<String>) -> Self {
        Self {
            semver_compatibility,
        }
    }
}

pub(super) async fn nodes_basic(
    state: State<AxumAppState>,
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

    Err(AxumErrorResponse::not_implemented())
}

pub(super) async fn nodes_expanded(
    state: State<AxumAppState>,
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

pub(super) async fn nodes_detailed(
    state: State<AxumAppState>,
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

pub(super) async fn gateways_basic(
    state: State<AxumAppState>,
    Query(SemverCompatibilityQueryParam {
        semver_compatibility,
    }): Query<SemverCompatibilityQueryParam>,
) -> AxumResult<Json<CachedNodesResponse<SkimmedNode>>> {
    let status_cache = state.node_status_cache();
    let describe_cache = state.described_nodes_state();
    let gateways_cache =
        status_cache
            .gateways_cache()
            .await
            .ok_or(AxumErrorResponse::internal_msg(
                "could not obtain gateways cache",
            ))?;

    if gateways_cache.is_empty() {
        return Ok(Json(CachedNodesResponse {
            refreshed_at: gateways_cache.timestamp().into(),
            nodes: vec![],
        }));
    }

    // if the self describe cache is unavailable don't try to use self-describe data
    let Ok(self_descriptions) = describe_cache.get().await else {
        return Ok(Json(CachedNodesResponse {
            refreshed_at: gateways_cache.timestamp().into(),
            nodes: gateways_cache.values().map(Into::into).collect(),
        }));
    };

    let refreshed_at = min(gateways_cache.timestamp(), self_descriptions.timestamp());

    // the same comment holds as with `get_gateways_described`.
    // this is inefficient and will have to get refactored with directory v3
    Ok(Json(CachedNodesResponse {
        refreshed_at: refreshed_at.into(),
        nodes: gateways_cache
            .values()
            .filter(|annotated_bond| {
                if let Some(semver_compatibility) = semver_compatibility.as_ref() {
                    version_checker::is_minor_version_compatible(
                        &annotated_bond.gateway_bond.gateway.version,
                        semver_compatibility,
                    )
                } else {
                    true
                }
            })
            .map(|annotated_bond| {
                SkimmedNode::from_described_gateway(
                    annotated_bond,
                    self_descriptions.deref().get(annotated_bond.identity()),
                )
            })
            .collect(),
    }))
}

pub(super) async fn gateways_expanded(
    State(_state): State<AxumAppState>,
    Query(SemverCompatibilityQueryParam {
        semver_compatibility: _semver_compatibility,
    }): Query<SemverCompatibilityQueryParam>,
) -> AxumResult<Json<CachedNodesResponse<SemiSkimmedNode>>> {
    Err(AxumErrorResponse::not_implemented())
}

pub(super) async fn gateways_detailed(
    State(_state): State<AxumAppState>,
    Query(SemverCompatibilityQueryParam {
        semver_compatibility: _semver_compatibility,
    }): Query<SemverCompatibilityQueryParam>,
) -> AxumResult<Json<CachedNodesResponse<FullFatNode>>> {
    Err(AxumErrorResponse::not_implemented())
}

pub(super) async fn mixnodes_basic(
    state: State<AxumAppState>,
    Query(SemverCompatibilityQueryParam {
        semver_compatibility,
    }): Query<SemverCompatibilityQueryParam>,
) -> AxumResult<Json<CachedNodesResponse<SkimmedNode>>> {
    let mixnodes_cache = state
        .node_status_cache()
        .active_mixnodes_cache()
        .await
        .ok_or(AxumErrorResponse::internal_msg(
            "could not obtain mixnodes cache",
        ))?;
    Ok(Json(CachedNodesResponse {
        refreshed_at: mixnodes_cache.timestamp().into(),
        nodes: mixnodes_cache
            .iter()
            .filter(|annotated_bond| {
                if let Some(semver_compatibility) = semver_compatibility.as_ref() {
                    version_checker::is_minor_version_compatible(
                        &annotated_bond
                            .mixnode_details
                            .bond_information
                            .mix_node
                            .version,
                        semver_compatibility,
                    )
                } else {
                    true
                }
            })
            .map(Into::into)
            .collect(),
    }))
}

pub(super) async fn mixnodes_expanded(
    State(_state): State<AxumAppState>,
    Query(SemverCompatibilityQueryParam {
        semver_compatibility: _semver_compatibility,
    }): Query<SemverCompatibilityQueryParam>,
) -> AxumResult<Json<CachedNodesResponse<SemiSkimmedNode>>> {
    Err(AxumErrorResponse::not_implemented())
}

pub(super) async fn mixnodes_detailed(
    State(_state): State<AxumAppState>,
    Query(SemverCompatibilityQueryParam {
        semver_compatibility: _semver_compatibility,
    }): Query<SemverCompatibilityQueryParam>,
) -> AxumResult<Json<CachedNodesResponse<FullFatNode>>> {
    Err(AxumErrorResponse::not_implemented())
}
