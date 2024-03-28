// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_describe_cache::DescribedNodes;
use crate::node_status_api::models::RocketErrorResponse;
use crate::node_status_api::NodeStatusCache;
use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::caching::cache::SharedCache;
use nym_api_requests::nym_nodes::{
    CachedNodesResponse, FullFatNode, NodeRole, NodeRoleQueryParam, SemiSkimmedNode, SkimmedNode,
};
use nym_bin_common::version_checker;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use std::collections::HashSet;

/*
   routes:

   // all routes/nodes are split into three tiers:
   // /skimmed      => [used by clients]            returns the very basic information for routing purposes
   // /semi-skimmed => [used by other nodes/VPN]    returns more additional information such noise keys
   // /full-fat     => [used by explorers, et al.]  returns almost everything there is about the nodes

   // there's also additional split based on the role:
   ?role => filters based on the specific role (mixnode/gateway/(in the future: entry/exit))
   /mixnodes/<tier> => only returns mixnode role data
   /gateway/<tier> => only returns (entry) gateway role data


*/

#[openapi(tag = "Unstable Nym Nodes")]
#[get("/skimmed?<role>&<semver_compatibility>")]
pub async fn nodes_basic(
    status_cache: &State<NodeStatusCache>,
    contract_cache: &State<NymContractCache>,
    describe_cache: &State<SharedCache<DescribedNodes>>,
    role: Option<NodeRoleQueryParam>,
    semver_compatibility: Option<String>,
) -> Result<Json<CachedNodesResponse<SkimmedNode>>, RocketErrorResponse> {
    if let Some(role) = role {
        match role {
            NodeRoleQueryParam::ActiveMixnode => {
                return mixnodes_basic(
                    status_cache,
                    contract_cache,
                    describe_cache,
                    semver_compatibility,
                )
                .await
            }
            NodeRoleQueryParam::EntryGateway => {
                return gateways_basic(
                    status_cache,
                    contract_cache,
                    describe_cache,
                    semver_compatibility,
                )
                .await
            }
            _ => {}
        }
    }

    Err(RocketErrorResponse::new(
        "unimplemented",
        Status::NotImplemented,
    ))
}

#[openapi(tag = "Unstable Nym Nodes")]
#[get("/semi-skimmed?<role>&<semver_compatibility>")]
pub async fn nodes_expanded(
    cache: &State<NodeStatusCache>,
    role: Option<NodeRoleQueryParam>,
    semver_compatibility: Option<String>,
) -> Result<Json<CachedNodesResponse<SemiSkimmedNode>>, RocketErrorResponse> {
    if let Some(role) = role {
        match role {
            NodeRoleQueryParam::ActiveMixnode => {
                return mixnodes_expanded(cache, semver_compatibility).await
            }
            NodeRoleQueryParam::EntryGateway => {
                return gateways_expanded(cache, semver_compatibility).await
            }
            _ => {}
        }
    }

    Err(RocketErrorResponse::new(
        "unimplemented",
        Status::NotImplemented,
    ))
}

#[openapi(tag = "Unstable Nym Nodes")]
#[get("/full-fat?<role>&<semver_compatibility>")]
pub async fn nodes_detailed(
    cache: &State<NodeStatusCache>,
    role: Option<NodeRoleQueryParam>,
    semver_compatibility: Option<String>,
) -> Result<Json<CachedNodesResponse<FullFatNode>>, RocketErrorResponse> {
    if let Some(role) = role {
        match role {
            NodeRoleQueryParam::ActiveMixnode => {
                return mixnodes_detailed(cache, semver_compatibility).await
            }
            NodeRoleQueryParam::EntryGateway => {
                return gateways_detailed(cache, semver_compatibility).await
            }
            _ => {}
        }
    }

    Err(RocketErrorResponse::new(
        "unimplemented",
        Status::NotImplemented,
    ))
}

#[openapi(tag = "Unstable Nym Nodes")]
#[get("/gateways/skimmed?<semver_compatibility>")]
pub async fn gateways_basic(
    status_cache: &State<NodeStatusCache>,
    contract_cache: &State<NymContractCache>,
    describe_cache: &State<SharedCache<DescribedNodes>>,
    semver_compatibility: Option<String>,
) -> Result<Json<CachedNodesResponse<SkimmedNode>>, RocketErrorResponse> {
    // 1. get the rewarded set
    let rewarded_set = contract_cache
        .rewarded_set()
        .await
        .ok_or_else(RocketErrorResponse::internal_server_error)?;

    // determine which gateways are active, i.e. which gateways the clients should be using for connecting and routing the traffic
    let active_gateways = rewarded_set.gateways().into_iter().collect::<HashSet<_>>();

    // 2. grab all annotations so that we could attach scores to the [nym] nodes
    let annotations = status_cache
        .node_annotations()
        .await
        .ok_or_else(RocketErrorResponse::internal_server_error)?;

    // 3. grab all legacy gateways
    // due to legacy endpoints we already have fully annotated data on them
    let annotated_legacy_gateways = status_cache
        .annotated_legacy_gateways()
        .await
        .ok_or_else(RocketErrorResponse::internal_server_error)?;

    // 4. grab all relevant described nym-nodes
    let describe_cache = describe_cache.get().await?;
    let gateway_capable_nym_nodes = describe_cache.gateway_capable_nym_nodes();

    // 5. only return nodes that are present in the active set
    let mut active_skimmed_gateways = Vec::new();

    for (node_id, legacy) in annotated_legacy_gateways.iter() {
        if !active_gateways.contains(node_id) {
            continue;
        }

        if let Some(semver_compat) = semver_compatibility.as_ref() {
            let version = legacy.version();
            if !version_checker::is_minor_version_compatible(version, semver_compat) {
                continue;
            }
        }

        // if we have self-described info, prefer it over contract data
        if let Some(described) = describe_cache.get_node(node_id) {
            active_skimmed_gateways.push(
                described.to_skimmed_node(NodeRole::EntryGateway, legacy.node_performance.last_24h),
            )
        } else {
            active_skimmed_gateways.push(legacy.into());
        }
    }

    for nym_node in gateway_capable_nym_nodes {
        // if this node is not an active gateway, ignore it
        if !active_gateways.contains(&nym_node.node_id) {
            continue;
        }

        // if we have wrong version, ignore
        if let Some(semver_compat) = semver_compatibility.as_ref() {
            let version = nym_node.version();
            if !version_checker::is_minor_version_compatible(version, semver_compat) {
                continue;
            }
        }

        // NOTE: if we determined our node IS an active gateway, it MUST be EITHER entry or exit
        let role = if rewarded_set.is_exit(&nym_node.node_id) {
            NodeRole::ExitGateway
        } else {
            NodeRole::EntryGateway
        };

        // honestly, not sure under what exact circumstances this value could be missing,
        // but in that case just use 0 performance
        let annotation = annotations
            .get(&nym_node.node_id)
            .copied()
            .unwrap_or_default();

        active_skimmed_gateways
            .push(nym_node.to_skimmed_node(role, annotation.last_24h_performance));
    }

    // min of all caches
    let refreshed_at = [
        rewarded_set.timestamp(),
        annotations.timestamp(),
        describe_cache.timestamp(),
        annotated_legacy_gateways.timestamp(),
    ]
    .into_iter()
    .min()
    .unwrap()
    .into();

    Ok(Json(CachedNodesResponse {
        refreshed_at,
        nodes: active_skimmed_gateways,
    }))
}

#[openapi(tag = "Unstable Nym Nodes")]
#[get("/gateways/semi-skimmed?<semver_compatibility>")]
pub async fn gateways_expanded(
    cache: &State<NodeStatusCache>,
    semver_compatibility: Option<String>,
) -> Result<Json<CachedNodesResponse<SemiSkimmedNode>>, RocketErrorResponse> {
    let _ = cache;
    let _ = semver_compatibility;
    Err(RocketErrorResponse::new(
        "unimplemented",
        Status::NotImplemented,
    ))
}

#[openapi(tag = "Unstable Nym Nodes")]
#[get("/gateways/full-fat?<semver_compatibility>")]
pub async fn gateways_detailed(
    cache: &State<NodeStatusCache>,
    semver_compatibility: Option<String>,
) -> Result<Json<CachedNodesResponse<FullFatNode>>, RocketErrorResponse> {
    let _ = cache;
    let _ = semver_compatibility;
    Err(RocketErrorResponse::new(
        "unimplemented",
        Status::NotImplemented,
    ))
}

#[openapi(tag = "Unstable Nym Nodes")]
#[get("/mixnodes/skimmed?<semver_compatibility>")]
pub async fn mixnodes_basic(
    status_cache: &State<NodeStatusCache>,
    contract_cache: &State<NymContractCache>,
    describe_cache: &State<SharedCache<DescribedNodes>>,
    semver_compatibility: Option<String>,
) -> Result<Json<CachedNodesResponse<SkimmedNode>>, RocketErrorResponse> {
    // 1. get the rewarded set
    let rewarded_set = contract_cache
        .rewarded_set()
        .await
        .ok_or_else(RocketErrorResponse::internal_server_error)?;

    // determine which mixnodes are active, i.e. which mixnodes the clients should be using for routing the traffic
    let active_mixnodes = rewarded_set
        .active_mixnodes()
        .into_iter()
        .collect::<HashSet<_>>();

    // 2. grab all annotations so that we could attach scores to the [nym] nodes
    let annotations = status_cache
        .node_annotations()
        .await
        .ok_or_else(RocketErrorResponse::internal_server_error)?;

    // 3. grab all legacy mixnodes
    // due to legacy endpoints we already have fully annotated data on them
    let annotated_legacy_mixnodes = status_cache
        .annotated_legacy_mixnodes()
        .await
        .ok_or_else(RocketErrorResponse::internal_server_error)?;

    // 4. grab all relevant described nym-nodes
    let describe_cache = describe_cache.get().await?;
    let mixing_nym_nodes = describe_cache.mixing_nym_nodes();

    // TODO: in the future, only use the self-described cache and simply reject mixnodes that did not expose it

    // 5. only return nodes that are present in the active set
    let mut active_skimmed_mixnodes = Vec::new();

    for (mix_id, legacy) in annotated_legacy_mixnodes.iter() {
        if !active_mixnodes.contains(mix_id) {
            continue;
        }

        if let Some(semver_compat) = semver_compatibility.as_ref() {
            let version = legacy.version();
            if !version_checker::is_minor_version_compatible(version, semver_compat) {
                continue;
            }
        }

        // if we have self-described info, prefer it over contract data
        if let Some(described) = describe_cache.get_node(mix_id) {
            active_skimmed_mixnodes.push(described.to_skimmed_node(
                NodeRole::Mixnode {
                    layer: legacy.mixnode_details.bond_information.layer.into(),
                },
                legacy.node_performance.last_24h,
            ))
        } else {
            active_skimmed_mixnodes.push(legacy.into());
        }
    }

    for nym_node in mixing_nym_nodes {
        // if this node is not an active mixnode, ignore it
        if !active_mixnodes.contains(&nym_node.node_id) {
            continue;
        }

        // if we have wrong version, ignore
        if let Some(semver_compat) = semver_compatibility.as_ref() {
            let version = nym_node.version();
            if !version_checker::is_minor_version_compatible(version, semver_compat) {
                continue;
            }
        }

        // SAFETY: if we determined our node IS active, it MUST have a layer
        // no other thread could have updated the rewarded set as we're still holding the [read] lock on the data
        #[allow(clippy::unwrap_used)]
        let layer = rewarded_set.try_get_mix_layer(&nym_node.node_id).unwrap();

        // honestly, not sure under what exact circumstances this value could be missing,
        // but in that case just use 0 performance
        let annotation = annotations
            .get(&nym_node.node_id)
            .copied()
            .unwrap_or_default();

        active_skimmed_mixnodes.push(
            nym_node.to_skimmed_node(NodeRole::Mixnode { layer }, annotation.last_24h_performance),
        );
    }

    // min of all caches
    let refreshed_at = [
        rewarded_set.timestamp(),
        annotations.timestamp(),
        describe_cache.timestamp(),
        annotated_legacy_mixnodes.timestamp(),
    ]
    .into_iter()
    .min()
    .unwrap()
    .into();

    Ok(Json(CachedNodesResponse {
        refreshed_at,
        nodes: active_skimmed_mixnodes,
    }))
}

#[openapi(tag = "Unstable Nym Nodes")]
#[get("/mixnodes/semi-skimmed?<semver_compatibility>")]
pub async fn mixnodes_expanded(
    cache: &State<NodeStatusCache>,
    semver_compatibility: Option<String>,
) -> Result<Json<CachedNodesResponse<SemiSkimmedNode>>, RocketErrorResponse> {
    let _ = cache;
    let _ = semver_compatibility;
    Err(RocketErrorResponse::new(
        "unimplemented",
        Status::NotImplemented,
    ))
}

#[openapi(tag = "Unstable Nym Nodes")]
#[get("/mixnodes/full-fat?<semver_compatibility>")]
pub async fn mixnodes_detailed(
    cache: &State<NodeStatusCache>,
    semver_compatibility: Option<String>,
) -> Result<Json<CachedNodesResponse<FullFatNode>>, RocketErrorResponse> {
    let _ = cache;
    let _ = semver_compatibility;
    Err(RocketErrorResponse::new(
        "unimplemented",
        Status::NotImplemented,
    ))
}
