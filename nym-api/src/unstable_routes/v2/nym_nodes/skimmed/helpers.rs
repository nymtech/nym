// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::nym_nodes::node_view::NodeSubset;
use crate::support::attested_snapshots::data::RetainedHeight;
use crate::support::http::state::AppState;
use crate::unstable_routes::helpers::build_nym_nodes_response;
use crate::unstable_routes::v2::nym_nodes::helpers::NodesParams;
use crate::unstable_routes::v2::nym_nodes::skimmed::PaginatedSkimmedNodes;
use axum::extract::{Query, State};
use nym_api_requests::nym_nodes::{PaginatedCachedNodesResponseV2, SkimmedNodeV1};
use nym_http_api_common::Output;

fn maybe_add_expires_header(
    output: Output,
    snapshot: &RetainedHeight,
    settle_lag: usize,
    nodes: Vec<SkimmedNodeV1>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    let mixnet = snapshot.mixnet();
    let base_response = output.to_response(
        PaginatedCachedNodesResponseV2::new_full(
            mixnet.current_interval.current_epoch_absolute_id(),
            mixnet.current_key_rotation_id(),
            snapshot.block_time(),
            nodes,
        )
        .fresh(mixnet.current_interval),
    );

    if !active_only {
        return Ok(base_response);
    }

    // the active set changes only with the epoch
    Ok(base_response.with_expires_header(snapshot.active_set_expiry(settle_lag)))
}

/// The `subset` nodes of the latest settled snapshot, or no nodes when the client already holds
/// the snapshot's epoch.
async fn build_skimmed_nodes_response(
    state: &AppState,
    query_params: NodesParams,
    subset: NodeSubset,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    // TODO: implement it
    let _ = query_params.per_page;
    let _ = query_params.page;
    let output = query_params.output.unwrap_or_default();

    let snapshot = state.latest_snapshot().await?;
    let interval = snapshot.mixnet().current_interval;
    let current_key_rotation = snapshot.mixnet().current_key_rotation_id();

    // If the client indicates that they already know about the current topology send empty response
    if let Some(client_known_epoch) = query_params.epoch_id {
        if client_known_epoch == interval.current_epoch_id() {
            return Ok(
                output.to_response(PaginatedCachedNodesResponseV2::no_updates(
                    interval.current_epoch_absolute_id(),
                    current_key_rotation,
                    snapshot.block_time(),
                )),
            );
        }
    }

    let nodes = build_nym_nodes_response(&snapshot, subset, active_only);

    maybe_add_expires_header(
        output,
        &snapshot,
        state.contract_snapshots.settle_lag(),
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

    let snapshot = state.latest_snapshot().await?;
    let mixnet = snapshot.mixnet();
    let nodes = build_nym_nodes_response(&snapshot, NodeSubset::All, active_only);

    Ok(output.to_response(PaginatedCachedNodesResponseV2::new_full(
        mixnet.current_interval.current_epoch_absolute_id(),
        mixnet.current_key_rotation_id(),
        snapshot.block_time(),
        nodes,
    )))
}

pub(crate) async fn mixnodes_basic(
    state: State<AppState>,
    Query(query_params): Query<NodesParams>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    build_skimmed_nodes_response(&state, query_params, NodeSubset::Mixing, active_only).await
}

pub(crate) async fn entry_gateways_basic(
    state: State<AppState>,
    Query(query_params): Query<NodesParams>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    build_skimmed_nodes_response(&state, query_params, NodeSubset::EntryCapable, active_only).await
}

pub(crate) async fn exit_gateways_basic(
    state: State<AppState>,
    Query(query_params): Query<NodesParams>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    build_skimmed_nodes_response(&state, query_params, NodeSubset::ExitCapable, active_only).await
}
