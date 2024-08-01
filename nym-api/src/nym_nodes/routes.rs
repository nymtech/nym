// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_describe_cache::DescribedNodes;
use crate::node_status_api::NodeStatusCache;
use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::caching::cache::SharedCache;
use nym_api_requests::models::{
    AnnotationResponse, LegacyDescribedGateway, LegacyDescribedMixNode, NymNodeDescription,
};
use nym_mixnet_contract_common::NodeId;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use std::ops::Deref;

#[openapi(tag = "Nym Nodes")]
#[get("/all/described")]
pub async fn all_described_nodes(
    describe_cache: &State<SharedCache<DescribedNodes>>,
) -> Json<Vec<NymNodeDescription>> {
    let Ok(self_descriptions) = describe_cache.get().await else {
        return Json(Vec::new());
    };

    Json(self_descriptions.all_nodes().cloned().collect())
}

#[openapi(tag = "Nym Nodes")]
#[get("/all/<node_id>/described")]
pub async fn node_description(
    node_id: NodeId,
    describe_cache: &State<SharedCache<DescribedNodes>>,
) -> Json<Option<NymNodeDescription>> {
    let Ok(self_descriptions) = describe_cache.get().await else {
        return Json(None);
    };

    Json(self_descriptions.get_node(&node_id).cloned())
}

#[openapi(tag = "Nym Nodes")]
#[get("/annotation-by-identity/<identity>")]
pub async fn node_annotation_by_identity(
    identity: String,
    status_cache: &State<NodeStatusCache>,
) -> Json<AnnotationResponse> {
    let Some(node_id) = status_cache.map_identity_to_node_id(&identity).await else {
        return Json(Default::default());
    };
    node_annotation(node_id, status_cache).await
}

#[openapi(tag = "Nym Nodes")]
#[get("/annotation/<node_id>")]
pub async fn node_annotation(
    node_id: NodeId,
    status_cache: &State<NodeStatusCache>,
) -> Json<AnnotationResponse> {
    let Some(annotation) = status_cache.node_annotations().await else {
        return Json(Default::default());
    };

    Json(AnnotationResponse {
        node_id: Some(node_id),
        annotation: annotation.get(&node_id).cloned(),
    })
}

/// This only returns descriptions of **legacy** gateways
#[openapi(tag = "Nym Nodes", deprecated = true)]
#[get("/gateways/described")]
pub async fn get_gateways_described(
    contract_cache: &State<NymContractCache>,
    describe_cache: &State<SharedCache<DescribedNodes>>,
) -> Json<Vec<LegacyDescribedGateway>> {
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
                self_described: self_descriptions
                    .deref()
                    .get_description(&bond.node_id)
                    .cloned(),
                bond,
            })
            .collect(),
    )
}

/// This only returns descriptions of **legacy** mixnodes
#[openapi(tag = "Nym Nodes", deprecated = true)]
#[get("/mixnodes/described")]
pub async fn get_mixnodes_described(
    contract_cache: &State<NymContractCache>,
    describe_cache: &State<SharedCache<DescribedNodes>>,
) -> Json<Vec<LegacyDescribedMixNode>> {
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
                self_described: self_descriptions
                    .deref()
                    .get_description(&bond.mix_id)
                    .cloned(),
                bond,
            })
            .collect(),
    )
}
