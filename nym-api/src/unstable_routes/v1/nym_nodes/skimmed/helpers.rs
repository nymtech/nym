// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::http::state::AppState;
use crate::unstable_routes::v1::nym_nodes::helpers::NodesParams;
use crate::unstable_routes::v1::nym_nodes::skimmed::PaginatedSkimmedNodes;
use crate::unstable_routes::v2;
use axum::extract::{Query, State};

pub(crate) async fn nodes_basic(
    state: State<AppState>,
    Query(query_params): Query<NodesParams>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    Ok(
        v2::nym_nodes::skimmed::helpers::nodes_basic(
            state,
            Query(query_params.into()),
            active_only,
        )
        .await?
        .map(Into::into),
    )
}

pub(crate) async fn mixnodes_basic(
    state: State<AppState>,
    Query(query_params): Query<NodesParams>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    Ok(v2::nym_nodes::skimmed::helpers::mixnodes_basic(
        state,
        Query(query_params.into()),
        active_only,
    )
    .await?
    .map(Into::into))
}

pub(crate) async fn entry_gateways_basic(
    state: State<AppState>,
    Query(query_params): Query<NodesParams>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    Ok(v2::nym_nodes::skimmed::helpers::entry_gateways_basic(
        state,
        Query(query_params.into()),
        active_only,
    )
    .await?
    .map(Into::into))
}

pub(crate) async fn exit_gateways_basic(
    state: State<AppState>,
    query_params: Query<NodesParams>,
    active_only: bool,
) -> PaginatedSkimmedNodes {
    Ok(v2::nym_nodes::skimmed::helpers::exit_gateways_basic(
        state,
        Query(query_params.0.into()),
        active_only,
    )
    .await?
    .map(Into::into))
}
