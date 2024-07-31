// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_describe_cache::DescribedNodes;
use crate::node_status_api::models::RocketErrorResponse;
use crate::node_status_api::NodeStatusCache;
use crate::support::caching::cache::SharedCache;
use nym_api_requests::nym_nodes::{
    CachedNodesResponse, FullFatNode, NodeRoleQueryParam, SemiSkimmedNode, SkimmedNode,
};
use nym_bin_common::version_checker;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use std::cmp::min;
use std::ops::Deref;

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
    describe_cache: &State<SharedCache<DescribedNodes>>,
    role: Option<NodeRoleQueryParam>,
    semver_compatibility: Option<String>,
) -> Result<Json<CachedNodesResponse<SkimmedNode>>, RocketErrorResponse> {
    if let Some(role) = role {
        match role {
            NodeRoleQueryParam::ActiveMixnode => {
                return mixnodes_basic(status_cache, semver_compatibility).await
            }
            NodeRoleQueryParam::EntryGateway => {
                return gateways_basic(status_cache, describe_cache, semver_compatibility).await
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
    describe_cache: &State<SharedCache<DescribedNodes>>,
    semver_compatibility: Option<String>,
) -> Result<Json<CachedNodesResponse<SkimmedNode>>, RocketErrorResponse> {
    let gateways_cache = status_cache
        .gateways_cache()
        .await
        .ok_or(RocketErrorResponse::new(
            "could not obtain gateways cache",
            Status::InternalServerError,
        ))?;

    if gateways_cache.is_empty() {
        return Ok(Json(CachedNodesResponse {
            refreshed_at: gateways_cache.timestamp().into(),
            nodes: vec![],
        }));
    }

    // if the self describe cache is unavailable don't try to use self-describe data
    let Ok(self_descriptions) = describe_cache.get().await else {
        return Ok(Json(CachedNodesResponse {
            refreshed_at: gateways_cache.timestamp().into(),
            nodes: gateways_cache.values().map(Into::into).collect(),
        }));
    };

    let refreshed_at = min(gateways_cache.timestamp(), self_descriptions.timestamp());

    // the same comment holds as with `get_gateways_described`.
    // this is inefficient and will have to get refactored with directory v3
    Ok(Json(CachedNodesResponse {
        refreshed_at: refreshed_at.into(),
        nodes: gateways_cache
            .values()
            .filter(|annotated_bond| {
                if let Some(semver_compatibility) = semver_compatibility.as_ref() {
                    version_checker::is_minor_version_compatible(
                        &annotated_bond.gateway_bond.gateway.version,
                        semver_compatibility,
                    )
                } else {
                    true
                }
            })
            .map(|annotated_bond| {
                SkimmedNode::from_described_gateway(
                    annotated_bond,
                    self_descriptions.deref().get(annotated_bond.identity()),
                )
            })
            .collect(),
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
    cache: &State<NodeStatusCache>,
    semver_compatibility: Option<String>,
) -> Result<Json<CachedNodesResponse<SkimmedNode>>, RocketErrorResponse> {
    let mixnodes_cache = cache
        .active_mixnodes_cache()
        .await
        .ok_or(RocketErrorResponse::new(
            "could not obtain mixnodes cache",
            Status::InternalServerError,
        ))?;
    Ok(Json(CachedNodesResponse {
        refreshed_at: mixnodes_cache.timestamp().into(),
        nodes: mixnodes_cache
            .iter()
            .filter(|annotated_bond| {
                if let Some(semver_compatibility) = semver_compatibility.as_ref() {
                    version_checker::is_minor_version_compatible(
                        &annotated_bond
                            .mixnode_details
                            .bond_information
                            .mix_node
                            .version,
                        semver_compatibility,
                    )
                } else {
                    true
                }
            })
            .map(Into::into)
            .collect(),
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
