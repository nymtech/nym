// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_describe_cache::DescribedNodes;
use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::nym_nodes::handlers::unstable::helpers::{refreshed_at, LegacyAnnotation};
use crate::nym_nodes::handlers::unstable::{NodesParams, NodesParamsWithRole};
use crate::support::caching::Cache;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use nym_api_requests::models::{
    NodeAnnotation, NymNodeDescription, OffsetDateTimeJsonSchemaWrapper,
};
use nym_api_requests::nym_nodes::{
    CachedNodesResponse, NodeRole, NodeRoleQueryParam, PaginatedCachedNodesResponse, SkimmedNode,
};
use nym_api_requests::pagination::PaginatedResponse;
use nym_http_api_common::{FormattedResponse, Output};
use nym_mixnet_contract_common::NodeId;
use nym_topology::CachedEpochRewardedSet;
use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;
use tokio::sync::RwLockReadGuard;
use tracing::trace;
use utoipa::ToSchema;

pub type PaginatedSkimmedNodes =
    AxumResult<FormattedResponse<PaginatedCachedNodesResponse<SkimmedNode>>>;

/// Given all relevant caches, build part of response for JUST Nym Nodes
fn build_nym_nodes_response<'a, NI>(
    rewarded_set: &CachedEpochRewardedSet,
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
    rewarded_set: &CachedEpochRewardedSet,
    describe_cache: &DescribedNodes,
    annotated_legacy_nodes: &HashMap<NodeId, LN>,
    active_only: bool,
) where
    LN: LegacyAnnotation,
{
    for (node_id, legacy) in annotated_legacy_nodes.iter() {
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
    output: Output,
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

    // 1. get the rewarded set
    let rewarded_set = state.rewarded_set().await?;

    // 2. grab all annotations so that we could attach scores to the [nym] nodes
    let annotations = state.node_annotations().await?;

    // 3. implicitly grab the relevant described nodes
    // (ideally it'd be tied directly to the NI iterator, but I couldn't defeat the compiler)
    let describe_cache = state.describe_nodes_cache_data().await?;

    let Some(interval) = state
        .nym_contract_cache()
        .current_interval()
        .await
        .to_owned()
    else {
        // if we can't obtain interval information, it means caches are not valid
        return Err(AxumErrorResponse::service_unavailable());
    };

    // 4.0 If the client indicates that they already know about the current topology send empty response
    if let Some(client_known_epoch) = query_params.epoch_id {
        if client_known_epoch == interval.current_epoch_id() {
            return Ok(output.to_response(PaginatedCachedNodesResponse::no_updates()));
        }
    }

    // 4. start building the response
    let mut nodes =
        build_nym_nodes_response(&rewarded_set, nym_nodes_subset, &annotations, active_only);

    // 5. if we allow legacy nodes, repeat the procedure for them, otherwise return just nym-nodes
    if let Some(true) = query_params.no_legacy {
        // min of all caches
        let refreshed_at = refreshed_at([
            rewarded_set.timestamp(),
            annotations.timestamp(),
            describe_cache.timestamp(),
        ]);

        return Ok(output.to_response(
            PaginatedCachedNodesResponse::new_full(refreshed_at, nodes).fresh(Some(interval)),
        ));
    }

    // 6. grab relevant legacy nodes
    // (due to the existence of the legacy endpoints, we already have fully annotated data on them)
    let annotated_legacy_nodes = annotated_legacy_nodes_getter(state).await?;
    add_legacy(
        &mut nodes,
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

    let base_response = output.to_response(
        PaginatedCachedNodesResponse::new_full(refreshed_at, nodes).fresh(Some(interval)),
    );

    if !active_only {
        return Ok(base_response);
    }

    // if caller requested only active nodes, the response is valid until the epoch changes
    // (but add 2 minutes due to epoch transition not being instantaneous
    let epoch_end = interval.current_epoch_end();
    let expiration = epoch_end + Duration::from_secs(120);
    Ok(base_response.with_expires_header(expiration))
}

/// Deprecated query that gets ALL gateways
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParams),
    path = "/gateways/skimmed",
    context_path = "/v1/unstable/nym-nodes",
    responses(
        (status = 200, content(
            (CachedNodesResponse<SkimmedNode> = "application/json"),
            (CachedNodesResponse<SkimmedNode> = "application/yaml"),
            (CachedNodesResponse<SkimmedNode> = "application/bincode")
        ))
    ),
)]
#[deprecated(note = "use '/v1/unstable/nym-nodes/entry-gateways/skimmed/all' instead")]
pub(super) async fn deprecated_gateways_basic(
    state: State<AppState>,
    query_params: Query<NodesParams>,
) -> AxumResult<FormattedResponse<CachedNodesResponse<SkimmedNode>>> {
    let output = query_params.output.unwrap_or_default();

    // 1. call '/v1/unstable/skimmed/entry-gateways/all'
    let all_gateways = entry_gateways_basic_all(state, query_params)
        .await?
        .into_inner();

    // 3. return result
    Ok(output.to_response(CachedNodesResponse {
        refreshed_at: all_gateways.refreshed_at,
        // 2. remove pagination
        nodes: all_gateways.nodes.data,
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
        (status = 200, content(
            (CachedNodesResponse<SkimmedNode> = "application/json"),
            (CachedNodesResponse<SkimmedNode> = "application/yaml"),
            (CachedNodesResponse<SkimmedNode> = "application/bincode")
        ))
    ),
)]
#[deprecated(note = "use '/v1/unstable/nym-nodes/skimmed/mixnodes/active' instead")]
pub(super) async fn deprecated_mixnodes_basic(
    state: State<AppState>,
    query_params: Query<NodesParams>,
) -> AxumResult<FormattedResponse<CachedNodesResponse<SkimmedNode>>> {
    let output = query_params.output.unwrap_or_default();

    // 1. call '/v1/unstable/nym-nodes/skimmed/mixnodes/active'
    let active_mixnodes = mixnodes_basic_active(state, query_params)
        .await?
        .into_inner();

    // 3. return result
    Ok(output.to_response(CachedNodesResponse {
        refreshed_at: active_mixnodes.refreshed_at,
        // 2. remove pagination
        nodes: active_mixnodes.nodes.data,
    }))
}

async fn nodes_basic(
    state: State<AppState>,
    Query(query_params): Query<NodesParams>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    let output = query_params.output.unwrap_or_default();

    // unfortunately we have to build the response semi-manually here as we need to add two sources of legacy nodes

    // 1. grab all relevant described nym-nodes
    let rewarded_set = state.rewarded_set().await?;

    let describe_cache = state.describe_nodes_cache_data().await?;
    let all_nym_nodes = describe_cache.all_nym_nodes();
    let annotations = state.node_annotations().await?;
    let legacy_mixnodes = state.legacy_mixnode_annotations().await?;
    let legacy_gateways = state.legacy_gateways_annotations().await?;

    let mut nodes =
        build_nym_nodes_response(&rewarded_set, all_nym_nodes, &annotations, active_only);

    // add legacy gateways to the response
    add_legacy(
        &mut nodes,
        &rewarded_set,
        &describe_cache,
        &legacy_gateways,
        active_only,
    );

    // add legacy mixnodes to the response
    add_legacy(
        &mut nodes,
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

    Ok(output.to_response(PaginatedCachedNodesResponse::new_full(refreshed_at, nodes)))
}

#[allow(dead_code)] // not dead, used in OpenAPI docs
#[derive(ToSchema)]
#[schema(title = "PaginatedCachedNodesResponse")]
pub struct PaginatedCachedNodesResponseSchema {
    pub refreshed_at: OffsetDateTimeJsonSchemaWrapper,
    #[schema(value_type = SkimmedNode)]
    pub nodes: PaginatedResponse<SkimmedNode>,
}

/// Return all Nym Nodes and optionally legacy mixnodes/gateways (if `no-legacy` flag is not used)
/// that are currently bonded.
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParamsWithRole),
    path = "",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, content(
            (PaginatedCachedNodesResponseSchema = "application/json"),
            (PaginatedCachedNodesResponseSchema = "application/yaml"),
            (PaginatedCachedNodesResponseSchema = "application/bincode")
        ))
    ),
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
    params(NodesParams),
    path = "/active",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, content(
            (PaginatedCachedNodesResponseSchema = "application/json"),
            (PaginatedCachedNodesResponseSchema = "application/yaml"),
            (PaginatedCachedNodesResponseSchema = "application/bincode")
        ))
    ),
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
    let output = query_params.output.unwrap_or_default();

    // 1. grab all relevant described nym-nodes
    let describe_cache = state.describe_nodes_cache_data().await?;
    let mixing_nym_nodes = describe_cache.mixing_nym_nodes();

    build_skimmed_nodes_response(
        &state.0,
        query_params,
        mixing_nym_nodes,
        |state| state.legacy_mixnode_annotations(),
        active_only,
        output,
    )
    .await
}

/// Returns Nym Nodes and optionally legacy mixnodes (if `no-legacy` flag is not used)
/// that are currently bonded and support mixing role.
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParams),
    path = "/mixnodes/all",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, content(
            (PaginatedCachedNodesResponseSchema = "application/json"),
            (PaginatedCachedNodesResponseSchema = "application/yaml"),
            (PaginatedCachedNodesResponseSchema = "application/bincode")
        ))
    ),
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
    params(NodesParams),
    path = "/mixnodes/active",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, content(
            (PaginatedCachedNodesResponseSchema = "application/json"),
            (PaginatedCachedNodesResponseSchema = "application/yaml"),
            (PaginatedCachedNodesResponseSchema = "application/bincode")
        ))
    ),
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
    let output = query_params.output.unwrap_or_default();

    // 1. grab all relevant described nym-nodes
    let describe_cache = state.describe_nodes_cache_data().await?;
    let mixing_nym_nodes = describe_cache.entry_capable_nym_nodes();

    build_skimmed_nodes_response(
        &state.0,
        query_params,
        mixing_nym_nodes,
        |state| state.legacy_gateways_annotations(),
        active_only,
        output,
    )
    .await
}

/// Returns Nym Nodes and optionally legacy gateways (if `no-legacy` flag is not used)
/// that are currently bonded and are in the active set with the entry role.
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParams),
    path = "/entry-gateways/active",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, content(
            (PaginatedCachedNodesResponseSchema = "application/json"),
            (PaginatedCachedNodesResponseSchema = "application/yaml"),
            (PaginatedCachedNodesResponseSchema = "application/bincode")
        ))
    ),
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
    params(NodesParams),
    path = "/entry-gateways/all",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, content(
            (PaginatedCachedNodesResponseSchema = "application/json"),
            (PaginatedCachedNodesResponseSchema = "application/yaml"),
            (PaginatedCachedNodesResponseSchema = "application/bincode")
        ))
    ),
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
    let output = query_params.output.unwrap_or_default();

    // 1. grab all relevant described nym-nodes
    let describe_cache = state.describe_nodes_cache_data().await?;
    let mixing_nym_nodes = describe_cache.exit_capable_nym_nodes();

    build_skimmed_nodes_response(
        &state.0,
        query_params,
        mixing_nym_nodes,
        |state| state.legacy_gateways_annotations(),
        active_only,
        output,
    )
    .await
}

/// Returns Nym Nodes and optionally legacy gateways (if `no-legacy` flag is not used)
/// that are currently bonded and are in the active set with the exit role.
#[utoipa::path(
    tag = "Unstable Nym Nodes",
    get,
    params(NodesParams),
    path = "/exit-gateways/active",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, content(
            (PaginatedCachedNodesResponseSchema = "application/json"),
            (PaginatedCachedNodesResponseSchema = "application/yaml"),
            (PaginatedCachedNodesResponseSchema = "application/bincode")
        ))
    ),
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
    params(NodesParams),
    path = "/exit-gateways/all",
    context_path = "/v1/unstable/nym-nodes/skimmed",
    responses(
        (status = 200, content(
            (PaginatedCachedNodesResponseSchema = "application/json"),
            (PaginatedCachedNodesResponseSchema = "application/yaml"),
            (PaginatedCachedNodesResponseSchema = "application/bincode")
        ))
    ),
)]
pub(super) async fn exit_gateways_basic_all(
    state: State<AppState>,
    query_params: Query<NodesParams>,
) -> PaginatedSkimmedNodes {
    exit_gateways_basic(state, query_params, false).await
}
