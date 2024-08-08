// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::v2::AxumAppState;
use axum::{extract::State, Json};
use nym_api_requests::models::DescribedGateway;
use std::ops::Deref;

// obviously this should get refactored later on because gateways will go away.
// unless maybe this will be filtering based on which nodes got assigned gateway role? TBD

pub(crate) async fn get_gateways_described(
    State(state): State<AxumAppState>,
) -> Json<Vec<DescribedGateway>> {
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
