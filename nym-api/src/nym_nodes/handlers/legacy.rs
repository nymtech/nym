// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::http::state::AppState;
use crate::support::legacy_helpers::{to_legacy_gateway, to_legacy_mixnode};
use axum::extract::{Query, State};
use axum::Router;
use nym_api_requests::models::{LegacyDescribedGateway, LegacyDescribedMixNode};
use nym_http_api_common::{FormattedResponse, OutputParams};
use tower_http::compression::CompressionLayer;

// we want to mark the routes as deprecated in swagger, but still expose them
#[allow(deprecated)]
pub(crate) fn legacy_nym_node_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/gateways/described",
            axum::routing::get(get_gateways_described),
        )
        .route(
            "/mixnodes/described",
            axum::routing::get(get_mixnodes_described),
        )
        .layer(CompressionLayer::new())
}

#[utoipa::path(
    tag = "Legacy gateways",
    get,
    path = "/v1/gateways/described",
    responses(
        (status = 200, content(
            (Vec<LegacyDescribedGateway> = "application/json"),
            (Vec<LegacyDescribedGateway> = "application/yaml"),
            (Vec<LegacyDescribedGateway> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
async fn get_gateways_described(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Vec<LegacyDescribedGateway>> {
    let describe_cache = state.described_nodes_cache();
    let output = output.output.unwrap_or_default();

    let Ok(describe_cache) = describe_cache.get().await else {
        return output.to_response(Vec::new());
    };

    let migrated_nymnodes = state.nym_contract_cache().nym_nodes().await;
    let mut described = Vec::new();

    for nym_node in migrated_nymnodes {
        // we ALWAYS need description to set legacy fields
        let Some(description) = describe_cache.get_description(&nym_node.node_id()) else {
            continue;
        };
        // if the node hasn't declared it can be a gateway, ignore it
        if !description.declared_role.entry {
            continue;
        }

        described.push(LegacyDescribedGateway {
            bond: to_legacy_gateway(&nym_node, description),
            self_described: Some(description.clone()),
        })
    }

    output.to_response(described)
}

#[utoipa::path(
    tag = "Legacy Mixnodes",
    get,
    path = "/v1/mixnodes/described",
    responses(
        (status = 200, content(
            (Vec<LegacyDescribedMixNode> = "application/json"),
            (Vec<LegacyDescribedMixNode> = "application/yaml"),
            (Vec<LegacyDescribedMixNode> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
async fn get_mixnodes_described(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Vec<LegacyDescribedMixNode>> {
    let describe_cache = state.described_nodes_cache();
    let output = output.output.unwrap_or_default();

    let Ok(describe_cache) = describe_cache.get().await else {
        return output.to_response(Vec::new());
    };

    let migrated_nymnodes = state.nym_contract_cache().nym_nodes().await;
    let mut described = Vec::new();

    for nym_node in migrated_nymnodes {
        // we ALWAYS need description to set legacy fields
        let Some(description) = describe_cache.get_description(&nym_node.node_id()) else {
            continue;
        };
        // if the node hasn't declared it can be a gateway, ignore it
        if !description.declared_role.mixnode {
            continue;
        }

        described.push(LegacyDescribedMixNode {
            bond: to_legacy_mixnode(&nym_node, description).bond_information,
            self_described: Some(description.clone()),
        })
    }

    output.to_response(described)
}
