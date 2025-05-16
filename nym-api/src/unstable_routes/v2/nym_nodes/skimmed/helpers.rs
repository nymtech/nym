// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_status_api::models::AxumErrorResponse;
use crate::support::caching::Cache;
use crate::support::http::state::AppState;
use crate::unstable_routes::helpers::{refreshed_at, LegacyAnnotation};
use crate::unstable_routes::v2::nym_nodes::helpers::NodesParams;
use crate::unstable_routes::v2::nym_nodes::skimmed::PaginatedSkimmedNodes;
use axum::extract::{Query, State};
use nym_api_requests::models::{
    NodeAnnotation, NymNodeDescription, OffsetDateTimeJsonSchemaWrapper,
};
use nym_api_requests::nym_nodes::{NodeRole, PaginatedCachedNodesResponseV2, SkimmedNode};
use nym_http_api_common::Output;
use nym_mixnet_contract_common::{Interval, NodeId};
use nym_topology::CachedEpochRewardedSet;
use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;
use tokio::sync::RwLockReadGuard;
use tracing::trace;

/// Given all relevant caches, build part of response for JUST Nym Nodes
fn build_nym_nodes_response<'a, NI>(
    rewarded_set: &CachedEpochRewardedSet,
    nym_nodes_subset: NI,
    annotations: &HashMap<NodeId, NodeAnnotation>,
    current_key_rotation: u32,
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

        nodes.push(nym_node.to_skimmed_node(
            current_key_rotation,
            role,
            annotation.last_24h_performance,
        ));
    }
    nodes
}

/// Given all relevant caches, add appropriate legacy nodes to the part of the response
fn add_legacy<LN>(
    nodes: &mut Vec<SkimmedNode>,
    rewarded_set: &CachedEpochRewardedSet,
    describe_cache: &DescribedNodes,
    annotated_legacy_nodes: &HashMap<NodeId, LN>,
    current_key_rotation: u32,
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
            // legacy nodes don't support key rotation
            nodes.push(described.to_skimmed_node(current_key_rotation, role, legacy.performance()))
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

fn maybe_add_expires_header(
    output: Output,
    interval: Interval,
    current_key_rotation: u32,
    refreshed_at: OffsetDateTimeJsonSchemaWrapper,
    nodes: Vec<SkimmedNode>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    let base_response = output.to_response(
        PaginatedCachedNodesResponseV2::new_full(
            interval.current_epoch_absolute_id(),
            current_key_rotation,
            refreshed_at,
            nodes,
        )
        .fresh(interval),
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

// hehe, what an abomination, but it's used in multiple different places and I hate copy-pasting code,
// especially if it has multiple loops, etc
pub(crate) async fn build_skimmed_nodes_response<'a, NI, LG, Fut, LN>(
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

    let contract_cache = state.nym_contract_cache();

    let interval = contract_cache.current_interval().await?;
    let current_key_rotation = contract_cache.current_key_rotation_id().await?;

    // 4.0 If the client indicates that they already know about the current topology send empty response
    if let Some(client_known_epoch) = query_params.epoch_id {
        if client_known_epoch == interval.current_epoch_id() {
            return Ok(
                output.to_response(PaginatedCachedNodesResponseV2::no_updates(
                    interval.current_epoch_absolute_id(),
                    current_key_rotation,
                )),
            );
        }
    }

    // 4. start building the response
    let mut nodes = build_nym_nodes_response(
        &rewarded_set,
        nym_nodes_subset,
        &annotations,
        current_key_rotation,
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

        return maybe_add_expires_header(
            output,
            interval,
            current_key_rotation,
            refreshed_at,
            nodes,
            active_only,
        );
    }

    // 6. grab relevant legacy nodes
    // (due to the existence of the legacy endpoints, we already have fully annotated data on them)
    let annotated_legacy_nodes = annotated_legacy_nodes_getter(state).await?;
    add_legacy(
        &mut nodes,
        &rewarded_set,
        &describe_cache,
        &annotated_legacy_nodes,
        current_key_rotation,
        active_only,
    );

    // min of all caches
    let refreshed_at = refreshed_at([
        rewarded_set.timestamp(),
        annotations.timestamp(),
        describe_cache.timestamp(),
        annotated_legacy_nodes.timestamp(),
    ]);

    maybe_add_expires_header(
        output,
        interval,
        current_key_rotation,
        refreshed_at,
        nodes,
        active_only,
    )
}

pub(crate) async fn nodes_basic(
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

    let interval = state.nym_contract_cache().current_interval().await?;
    let current_key_rotation = state.nym_contract_cache().current_key_rotation_id().await?;

    let mut nodes = build_nym_nodes_response(
        &rewarded_set,
        all_nym_nodes,
        &annotations,
        current_key_rotation,
        active_only,
    );

    // add legacy gateways to the response
    add_legacy(
        &mut nodes,
        &rewarded_set,
        &describe_cache,
        &legacy_gateways,
        current_key_rotation,
        active_only,
    );

    // add legacy mixnodes to the response
    add_legacy(
        &mut nodes,
        &rewarded_set,
        &describe_cache,
        &legacy_mixnodes,
        current_key_rotation,
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

    Ok(output.to_response(PaginatedCachedNodesResponseV2::new_full(
        current_key_rotation,
        interval.current_epoch_absolute_id(),
        refreshed_at,
        nodes,
    )))
}

pub(crate) async fn mixnodes_basic(
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

pub(crate) async fn entry_gateways_basic(
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

pub(crate) async fn exit_gateways_basic(
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
