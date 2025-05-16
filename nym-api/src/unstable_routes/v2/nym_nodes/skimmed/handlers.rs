// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::http::state::AppState;
use crate::unstable_routes::v2::nym_nodes::helpers::{NodesParams, NodesParamsWithRole};
use crate::unstable_routes::v2::nym_nodes::skimmed::helpers::{
    entry_gateways_basic, exit_gateways_basic, mixnodes_basic, nodes_basic,
};
use crate::unstable_routes::v2::nym_nodes::skimmed::{
    PaginatedCachedNodesResponseSchema, PaginatedSkimmedNodes,
};
use axum::extract::{Query, State};
use nym_api_requests::nym_nodes::NodeRoleQueryParam;

/// Return all Nym Nodes and optionally legacy mixnodes/gateways (if `no-legacy` flag is not used)
/// that are currently bonded.
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParamsWithRole),
    path = "",
    context_path = "/v2/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, content(
            (PaginatedCachedNodesResponseSchema = "application/json"),
            (PaginatedCachedNodesResponseSchema = "application/yaml"),
            (PaginatedCachedNodesResponseSchema = "application/bincode")
        ))
    ),
)]
pub(crate) async fn nodes_basic_all(
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

/// Returns Nym Nodes and optionally legacy mixnodes (if `no-legacy` flag is not used)
/// that are currently bonded and support mixing role.
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParams),
    path = "/mixnodes/all",
    context_path = "/v2/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, content(
            (PaginatedCachedNodesResponseSchema = "application/json"),
            (PaginatedCachedNodesResponseSchema = "application/yaml"),
            (PaginatedCachedNodesResponseSchema = "application/bincode")
        ))
    ),
)]
pub(crate) async fn mixnodes_basic_all(
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
    params(NodesParams),
    path = "/mixnodes/active",
    context_path = "/v2/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, content(
            (PaginatedCachedNodesResponseSchema = "application/json"),
            (PaginatedCachedNodesResponseSchema = "application/yaml"),
            (PaginatedCachedNodesResponseSchema = "application/bincode")
        ))
    ),
)]
pub(crate) async fn mixnodes_basic_active(
    state: State<AppState>,
    query_params: Query<NodesParams>,
) -> PaginatedSkimmedNodes {
    mixnodes_basic(state, query_params, true).await
}

/// Returns Nym Nodes and optionally legacy gateways (if `no-legacy` flag is not used)
/// that are currently bonded and support entry gateway role.
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParams),
    path = "/entry-gateways/all",
    context_path = "/v2/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, content(
            (PaginatedCachedNodesResponseSchema = "application/json"),
            (PaginatedCachedNodesResponseSchema = "application/yaml"),
            (PaginatedCachedNodesResponseSchema = "application/bincode")
        ))
    ),
)]
pub(crate) async fn entry_gateways_basic_all(
    state: State<AppState>,
    query_params: Query<NodesParams>,
) -> PaginatedSkimmedNodes {
    entry_gateways_basic(state, query_params, false).await
}

/// Returns Nym Nodes and optionally legacy gateways (if `no-legacy` flag is not used)
/// that are currently bonded and support exit gateway role.
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParams),
    path = "/exit-gateways/all",
    context_path = "/v2/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, content(
            (PaginatedCachedNodesResponseSchema = "application/json"),
            (PaginatedCachedNodesResponseSchema = "application/yaml"),
            (PaginatedCachedNodesResponseSchema = "application/bincode")
        ))
    ),
)]
pub(crate) async fn exit_gateways_basic_all(
    state: State<AppState>,
    query_params: Query<NodesParams>,
) -> PaginatedSkimmedNodes {
    exit_gateways_basic(state, query_params, false).await
}
