// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::v2::AxumAppState;
use axum::{extract::State, Json, Router};
use nym_api_requests::models::{DescribedGateway, DescribedMixNode};
use nym_mixnet_contract_common::MixNodeBond;
use std::ops::Deref;

// obviously this should get refactored later on because gateways will go away.
// unless maybe this will be filtering based on which nodes got assigned gateway role? TBD

pub(crate) fn nym_node_routes() -> axum::Router<AxumAppState> {
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
    tag = "Nym Nodes",
    get,
    path = "/v1/gateways/described",
    responses(
        (status = 200, body = Vec<DescribedGateway>)
    )
)]
async fn get_gateways_described(State(state): State<AxumAppState>) -> Json<Vec<DescribedGateway>> {
    let gateways = state.nym_contract_cache().gateways_filtered().await;
    if gateways.is_empty() {
        return Json(Vec::new());
    }

    // if the self describe cache is unavailable, well, don't attach describe data
    let Ok(self_descriptions) = state.described_nodes_state().get().await else {
        return Json(gateways.into_iter().map(Into::into).collect());
    };

    // TODO: this is extremely inefficient, but given we don't have many gateways,
    // it shouldn't be too much of a problem until we go ahead with directory v3 / the smoosh 2: electric smoosharoo,
    // but at that point (I hope) the whole caching situation should get refactored
    Json(
        gateways
            .into_iter()
            .map(|bond| DescribedGateway {
                self_described: self_descriptions.deref().get(bond.identity()).cloned(),
                bond,
            })
            .collect(),
    )
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/v1/mixnodes/described",
    responses(
        (status = 200, body = Vec<DescribedMixNode>)
    )
)]
async fn get_mixnodes_described(State(state): State<AxumAppState>) -> Json<Vec<DescribedMixNode>> {
    let mixnodes = state
        .nym_contract_cache()
        .mixnodes_filtered()
        .await
        .into_iter()
        .map(|m| m.bond_information)
        .collect::<Vec<MixNodeBond>>();
    if mixnodes.is_empty() {
        return Json(Vec::new());
    }

    // if the self describe cache is unavailable, well, don't attach describe data
    let Ok(self_descriptions) = state.described_nodes_state().get().await else {
        return Json(mixnodes.into_iter().map(Into::into).collect());
    };

    // TODO: this is extremely inefficient, but given we don't have many gateways,
    // it shouldn't be too much of a problem until we go ahead with directory v3 / the smoosh 2: electric smoosharoo,
    // but at that point (I hope) the whole caching situation should get refactored
    Json(
        mixnodes
            .into_iter()
            .map(|bond| DescribedMixNode {
                self_described: self_descriptions.deref().get(bond.identity()).cloned(),
                bond,
            })
            .collect(),
    )
}
