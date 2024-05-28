// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    node_status_api::{
        helpers::{_get_active_set_detailed, _get_mixnodes_detailed, _get_rewarded_set_detailed},
        NodeStatusCache,
    },
    nym_contract_cache::cache::NymContractCache,
};
use nym_api_requests::models::MixNodeBondAnnotated;
use nym_mixnet_contract_common::{
    mixnode::MixNodeDetails, reward_params::RewardingParams, GatewayBond, Interval, MixId,
};

use nym_name_service_common::response::NamesListResponse;
use nym_service_provider_directory_common::response::ServicesListResponse;
use rocket::{get, serde::json::Json, State};
use rocket_okapi::openapi;
use std::collections::HashSet;

#[openapi(tag = "contract-cache")]
#[get("/mixnodes")]
pub async fn get_mixnodes(cache: &State<NymContractCache>) -> Json<Vec<MixNodeDetails>> {
    Json(cache.mixnodes_filtered().await)
}

// DEPRECATED: this endpoint now lives in `node_status_api`. Once all consumers are updated,
// replace this with
// ```
//  pub fn get_mixnodes_detailed() -> Redirect {
//      Redirect::to(uri!("/v1/status/mixnodes/detailed"))
//  }
// ```
#[openapi(tag = "contract-cache")]
#[get("/mixnodes/detailed")]
pub async fn get_mixnodes_detailed(
    cache: &State<NodeStatusCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_mixnodes_detailed(cache).await)
}

#[openapi(tag = "contract-cache")]
#[get("/gateways")]
pub async fn get_gateways(cache: &State<NymContractCache>) -> Json<Vec<GatewayBond>> {
    Json(cache.gateways_filtered().await)
}

#[openapi(tag = "contract-cache")]
#[get("/mixnodes/rewarded")]
pub async fn get_rewarded_set(cache: &State<NymContractCache>) -> Json<Vec<MixNodeDetails>> {
    Json(cache.rewarded_set().await.clone())
}

// DEPRECATED: this endpoint now lives in `node_status_api`. Once all consumers are updated,
// replace this with
// ```
//  pub fn get_mixnodes_set_detailed() -> Redirect {
//      Redirect::to(uri!("/v1/status/mixnodes/rewarded/detailed"))
//  }
// ```
#[openapi(tag = "contract-cache")]
#[get("/mixnodes/rewarded/detailed")]
pub async fn get_rewarded_set_detailed(
    cache: &State<NodeStatusCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_rewarded_set_detailed(cache).await)
}

#[openapi(tag = "contract-cache")]
#[get("/mixnodes/active")]
pub async fn get_active_set(cache: &State<NymContractCache>) -> Json<Vec<MixNodeDetails>> {
    Json(cache.active_set().await.clone())
}

// DEPRECATED: this endpoint now lives in `node_status_api`. Once all consumers are updated,
// replace this with
// ```
//  pub fn get_active_set_detailed() -> Redirect {
//      Redirect::to(uri!("/status/mixnodes/active/detailed"))
//  }
// ```
#[openapi(tag = "contract-cache")]
#[get("/mixnodes/active/detailed")]
pub async fn get_active_set_detailed(
    cache: &State<NodeStatusCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_active_set_detailed(cache).await)
}

#[openapi(tag = "contract-cache")]
#[get("/mixnodes/blacklisted")]
pub async fn get_blacklisted_mixnodes(
    cache: &State<NymContractCache>,
) -> Json<Option<HashSet<MixId>>> {
    let blacklist = cache.mixnodes_blacklist().await.clone();
    if blacklist.is_empty() {
        Json(None)
    } else {
        Json(Some(blacklist))
    }
}

#[openapi(tag = "contract-cache")]
#[get("/gateways/blacklisted")]
pub async fn get_blacklisted_gateways(
    cache: &State<NymContractCache>,
) -> Json<Option<HashSet<String>>> {
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

#[openapi(tag = "contract-cache")]
#[get("/services")]
pub async fn get_services(cache: &State<NymContractCache>) -> Json<ServicesListResponse> {
    let services = cache.services().await.clone();
    Json(services.as_slice().into())
}

#[openapi(tag = "contract-cache")]
#[get("/names")]
pub async fn get_registered_names(cache: &State<NymContractCache>) -> Json<NamesListResponse> {
    let names = cache.names().await.clone();
    Json(names.as_slice().into())
}
