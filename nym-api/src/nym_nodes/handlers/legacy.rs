// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::http::state::AppState;
use crate::support::legacy_helpers::{to_legacy_gateway, to_legacy_mixnode};
use axum::extract::State;
use axum::{Json, Router};
use nym_api_requests::legacy::LegacyMixNodeBondWithLayer;
use nym_api_requests::models::{LegacyDescribedGateway, LegacyDescribedMixNode};
use nym_http_api_common::middleware::compression::new_compression_layer;

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
        .layer(new_compression_layer())
}

#[utoipa::path(
    tag = "Legacy gateways",
    get,
    path = "/v1/gateways/described",
    responses(
        (status = 200, body = Vec<LegacyDescribedGateway>)
    )
)]
#[deprecated]
async fn get_gateways_described(
    State(state): State<AppState>,
) -> Json<Vec<LegacyDescribedGateway>> {
    let contract_cache = state.nym_contract_cache();
    let describe_cache = state.described_nodes_cache();

    // legacy
    let legacy = contract_cache.legacy_gateways_filtered().await;

    // if the self describe cache is unavailable, well, don't attach describe data and only return legacy gateways
    let Ok(describe_cache) = describe_cache.get().await else {
        return Json(legacy.into_iter().map(Into::into).collect());
    };

    let migrated_nymnodes = state.nym_contract_cache().nym_nodes().await;
    let mut out = Vec::new();

    for legacy_bond in legacy {
        out.push(LegacyDescribedGateway {
            self_described: describe_cache
                .get_description(&legacy_bond.node_id)
                .cloned(),
            bond: legacy_bond.bond,
        })
    }

    for nym_node in migrated_nymnodes {
        // we ALWAYS need description to set legacy fields
        let Some(description) = describe_cache.get_description(&nym_node.node_id()) else {
            continue;
        };
        // if the node hasn't declared it can be a gateway, ignore it
        if !description.declared_role.entry {
            continue;
        }

        out.push(LegacyDescribedGateway {
            bond: to_legacy_gateway(&nym_node, description),
            self_described: Some(description.clone()),
        })
    }

    Json(out)
}

#[utoipa::path(
    tag = "Legacy Mixnodes",
    get,
    path = "/v1/mixnodes/described",
    responses(
        (status = 200, body = Vec<LegacyDescribedMixNode>)
    )
)]
#[deprecated]
async fn get_mixnodes_described(
    State(state): State<AppState>,
) -> Json<Vec<LegacyDescribedMixNode>> {
    let contract_cache = state.nym_contract_cache();
    let describe_cache = state.described_nodes_cache();

    let legacy: Vec<LegacyMixNodeBondWithLayer> = contract_cache
        .legacy_mixnodes_filtered()
        .await
        .into_iter()
        .map(|m| m.bond_information)
        .collect::<Vec<_>>();

    // if the self describe cache is unavailable, well, don't attach describe data and only return legacy mixnodes
    let Ok(describe_cache) = describe_cache.get().await else {
        return Json(legacy.into_iter().map(Into::into).collect());
    };

    let migrated_nymnodes = state.nym_contract_cache().nym_nodes().await;
    let mut out = Vec::new();

    for legacy_bond in legacy {
        out.push(LegacyDescribedMixNode {
            self_described: describe_cache.get_description(&legacy_bond.mix_id).cloned(),
            bond: legacy_bond,
        })
    }

    for nym_node in migrated_nymnodes {
        // we ALWAYS need description to set legacy fields
        let Some(description) = describe_cache.get_description(&nym_node.node_id()) else {
            continue;
        };
        // if the node hasn't declared it can be a gateway, ignore it
        if !description.declared_role.mixnode {
            continue;
        }

        out.push(LegacyDescribedMixNode {
            bond: to_legacy_mixnode(&nym_node, description).bond_information,
            self_described: Some(description.clone()),
        })
    }

    Json(out)
}
