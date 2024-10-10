// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::http::state::AppState;
use axum::extract::State;
use axum::{Json, Router};
use nym_api_requests::models::{LegacyDescribedGateway, LegacyDescribedMixNode};

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
}

#[utoipa::path(
    tag = "Legacy gateways",
    get,
    path = "/v1/gateways/described",
    responses(
        (status = 200, body = Vec<DescribedGateway>)
    )
)]
#[deprecated]
async fn get_gateways_described(
    State(state): State<AppState>,
) -> Json<Vec<LegacyDescribedGateway>> {
    let contract_cache = state.nym_contract_cache();
    let describe_cache = state.described_nodes_cache();
    let gateways = contract_cache.legacy_gateways_filtered().await;
    if gateways.is_empty() {
        return Json(Vec::new());
    }

    // if the self describe cache is unavailable, well, don't attach describe data and only return legacy gateways
    let Ok(self_descriptions) = describe_cache.get().await else {
        return Json(gateways.into_iter().map(Into::into).collect());
    };

    Json(
        gateways
            .into_iter()
            .map(|bond| LegacyDescribedGateway {
                self_described: self_descriptions.get_description(&bond.node_id).cloned(),
                bond,
            })
            .collect(),
    )
}

#[utoipa::path(
    tag = "Legacy Mixnodes",
    get,
    path = "/v1/mixnodes/described",
    responses(
        (status = 200, body = Vec<DescribedMixNode>)
    )
)]
#[deprecated]
async fn get_mixnodes_described(
    State(state): State<AppState>,
) -> Json<Vec<LegacyDescribedMixNode>> {
    let contract_cache = state.nym_contract_cache();
    let describe_cache = state.described_nodes_cache();

    let mixnodes = contract_cache
        .legacy_mixnodes_filtered()
        .await
        .into_iter()
        .map(|m| m.bond_information)
        .collect::<Vec<_>>();
    if mixnodes.is_empty() {
        return Json(Vec::new());
    }

    // if the self describe cache is unavailable, well, don't attach describe data
    let Ok(self_descriptions) = describe_cache.get().await else {
        return Json(mixnodes.into_iter().map(Into::into).collect());
    };

    Json(
        mixnodes
            .into_iter()
            .map(|bond| LegacyDescribedMixNode {
                self_described: self_descriptions.get_description(&bond.mix_id).cloned(),
                bond,
            })
            .collect(),
    )
}
