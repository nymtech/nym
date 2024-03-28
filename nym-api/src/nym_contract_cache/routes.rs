// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{node_status_api::NodeStatusCache, nym_contract_cache::cache::NymContractCache};
use nym_api_requests::models::MixNodeBondAnnotated;
use nym_mixnet_contract_common::{reward_params::RewardingParams, GatewayBond, Interval, NodeId};

use nym_api_requests::legacy::LegacyMixNodeDetailsWithLayer;
use rocket::{serde::json::Json, State};
use rocket_okapi::openapi;
use std::collections::HashSet;

#[openapi(tag = "contract-cache", deprecated = true)]
#[get("/mixnodes")]
pub async fn get_mixnodes(
    cache: &State<NymContractCache>,
) -> Json<Vec<LegacyMixNodeDetailsWithLayer>> {
    Json(cache.legacy_mixnodes_filtered().await)
}

// DEPRECATED: this endpoint now lives in `node_status_api`. Once all consumers are updated,
// replace this with
// ```
//  pub fn get_mixnodes_detailed() -> Redirect {
//      Redirect::to(uri!("/v1/status/mixnodes/detailed"))
//  }
// ```
#[openapi(tag = "contract-cache", deprecated = true)]
#[get("/mixnodes/detailed")]
pub async fn get_mixnodes_detailed(
    cache: &State<NodeStatusCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    todo!("rebasing")
    // Json(_get_legacy_mixnodes_detailed(cache).await)
}

#[openapi(tag = "contract-cache", deprecated = true)]
#[get("/gateways")]
pub async fn get_gateways(cache: &State<NymContractCache>) -> Json<Vec<GatewayBond>> {
    Json(
        cache
            .legacy_gateways_filtered()
            .await
            .into_iter()
            .map(Into::into)
            .collect(),
    )
}

#[openapi(tag = "contract-cache", deprecated = true)]
#[get("/mixnodes/rewarded")]
pub async fn get_rewarded_set(
    cache: &State<NymContractCache>,
) -> Json<Vec<LegacyMixNodeDetailsWithLayer>> {
    Json(cache.legacy_v1_rewarded_set_mixnodes().await.clone())
}

// DEPRECATED: this endpoint now lives in `node_status_api`. Once all consumers are updated,
// replace this with
// ```
//  pub fn get_mixnodes_set_detailed() -> Redirect {
//      Redirect::to(uri!("/v1/status/mixnodes/rewarded/detailed"))
//  }
// ```
#[openapi(tag = "contract-cache", deprecated = true)]
#[get("/mixnodes/rewarded/detailed")]
pub async fn get_rewarded_set_detailed(
    status_cache: &State<NodeStatusCache>,
    contract_cache: &State<NymContractCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    todo!("rebasing")
    // Json(_get_rewarded_set_legacy_mixnodes_detailed(status_cache, contract_cache).await)
}

#[openapi(tag = "contract-cache", deprecated = true)]
#[get("/mixnodes/active")]
pub async fn get_active_set(
    cache: &State<NymContractCache>,
) -> Json<Vec<LegacyMixNodeDetailsWithLayer>> {
    Json(cache.legacy_v1_active_set_mixnodes().await.clone())
}

// DEPRECATED: this endpoint now lives in `node_status_api`. Once all consumers are updated,
// replace this with
// ```
//  pub fn get_active_set_detailed() -> Redirect {
//      Redirect::to(uri!("/status/mixnodes/active/detailed"))
//  }
// ```
#[openapi(tag = "contract-cache", deprecated = true)]
#[get("/mixnodes/active/detailed")]
pub async fn get_active_set_detailed(
    status_cache: &State<NodeStatusCache>,
    contract_cache: &State<NymContractCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    todo!("rebasing")
    // Json(_get_active_set_legacy_mixnodes_detailed(status_cache, contract_cache).await)
}

#[openapi(tag = "contract-cache", deprecated = true)]
#[get("/mixnodes/blacklisted")]
pub async fn get_blacklisted_mixnodes(
    cache: &State<NymContractCache>,
) -> Json<Option<HashSet<NodeId>>> {
    let blacklist = cache.mixnodes_blacklist().await.clone();
    if blacklist.is_empty() {
        Json(None)
    } else {
        Json(Some(blacklist))
    }
}

#[openapi(tag = "contract-cache", deprecated = true)]
#[get("/gateways/blacklisted")]
pub async fn get_blacklisted_gateways(
    cache: &State<NymContractCache>,
) -> Json<Option<HashSet<String>>> {
    let blacklist = cache.gateways_blacklist().await.clone();
    if blacklist.is_empty() {
        Json(None)
    } else {
        let gateways = cache.legacy_gateways_all().await;
        Json(Some(
            gateways
                .into_iter()
                .filter(|g| blacklist.contains(&g.node_id))
                .map(|g| g.gateway.identity_key.clone())
                .collect(),
        ))
    }
}

#[openapi(tag = "contract-cache", deprecated = true)]
#[get("/gateways/blacklisted_v2")]
pub async fn get_blacklisted_gateways_v2(
    cache: &State<NymContractCache>,
) -> Json<Option<HashSet<NodeId>>> {
    let blacklist = cache.gateways_blacklist().await.clone();
    if blacklist.is_empty() {
        Json(None)
    } else {
        Json(Some(blacklist))
    }
}

#[openapi(tag = "contract-cache")]
#[get("/epoch/reward_params")]
pub async fn get_interval_reward_params(
    cache: &State<NymContractCache>,
) -> Json<Option<RewardingParams>> {
    Json(*cache.interval_reward_params().await)
}

#[openapi(tag = "contract-cache")]
#[get("/epoch/current")]
pub async fn get_current_epoch(cache: &State<NymContractCache>) -> Json<Option<Interval>> {
    Json(*cache.current_interval().await)
}
