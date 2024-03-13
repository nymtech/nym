// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_describe_cache::DescribedNodes;
use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::caching::cache::SharedCache;
use nym_api_requests::models::{DescribedGateway, DescribedMixnode, DescribedNymNode};
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use std::ops::Deref;

// obviously this should get refactored later on because gateways will go away.
// unless maybe this will be filtering based on which nodes got assigned gateway role? TBD

#[openapi(tag = "Nym Nodes")]
#[get("/gateways/described")]
pub async fn get_gateways_described(
    contract_cache: &State<NymContractCache>,
    describe_cache: &State<SharedCache<DescribedNodes>>,
) -> Json<Vec<DescribedGateway>> {
    let gateways = contract_cache.gateways_filtered().await;
    if gateways.is_empty() {
        return Json(Vec::new());
    }

    // if the self describe cache is unavailable, well, don't attach describe data
    let Ok(self_descriptions) = describe_cache.get().await else {
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

#[openapi(tag = "Nym Nodes")]
#[get("/nym-nodes/described")]
pub async fn get_nym_nodes_described(
    contract_cache: &State<NymContractCache>,
    describe_cache: &State<SharedCache<DescribedNodes>>,
) -> Json<Vec<DescribedNymNode>> {
    let gateways = contract_cache.gateways_all().await;
    let mixnodes = contract_cache.mixnodes_all().await;
    if gateways.is_empty() && mixnodes.is_empty() {
        return Json(Vec::new());
    }

    // if the self describe cache is unavailable, well, don't attach describe data
    let Ok(self_descriptions) = describe_cache.get().await else {
        return Json(
            gateways
                .into_iter()
                .map(Into::into)
                .chain(mixnodes.into_iter().map(Into::into))
                .collect(),
        );
    };

    // TODO: this is extremely inefficient, I'm merely copying existing stuff
    // it shouldn't be too much of a problem until we go ahead with directory v3 / the smoosh 2: electric smoosharoo,
    // but at that point (I hope) the whole caching situation should get refactored
    let gateways_described: Vec<DescribedNymNode> = gateways
        .into_iter()
        .map(|bond| {
            DescribedNymNode::Gateway(DescribedGateway {
                self_described: self_descriptions.deref().get(bond.identity()).cloned(),
                bond,
            })
        })
        .collect();

    let mixnodes_described: Vec<DescribedNymNode> = mixnodes
        .into_iter()
        .map(|detail| {
            DescribedNymNode::Mixnode(DescribedMixnode {
                self_described: self_descriptions
                    .deref()
                    .get(detail.bond_information.identity())
                    .cloned(),
                bond: detail.bond_information,
            })
        })
        .collect();
    Json(
        gateways_described
            .into_iter()
            .chain(mixnodes_described.into_iter())
            .collect(),
    )
}
