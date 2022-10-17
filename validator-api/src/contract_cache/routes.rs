// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract_cache::ValidatorCache;
use mixnet_contract_common::mixnode::MixNodeDetails;
use mixnet_contract_common::reward_params::RewardingParams;
use mixnet_contract_common::{GatewayBond, Interval, MixId};
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use std::collections::HashSet;
use validator_api_requests::models::MixNodeBondAnnotated;

#[openapi(tag = "contract-cache")]
#[get("/mixnodes")]
pub async fn get_mixnodes(cache: &State<ValidatorCache>) -> Json<Vec<MixNodeDetails>> {
    Json(cache.mixnodes().await)
}

#[openapi(tag = "contract-cache")]
#[get("/mixnodes/detailed")]
pub async fn get_mixnodes_detailed(
    cache: &State<ValidatorCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(cache.mixnodes_detailed().await)
}

#[openapi(tag = "contract-cache")]
#[get("/gateways")]
pub async fn get_gateways(cache: &State<ValidatorCache>) -> Json<Vec<GatewayBond>> {
    Json(cache.gateways().await)
}

#[openapi(tag = "contract-cache")]
#[get("/mixnodes/rewarded")]
pub async fn get_rewarded_set(cache: &State<ValidatorCache>) -> Json<Vec<MixNodeDetails>> {
    Json(cache.rewarded_set().await)
}

#[openapi(tag = "contract-cache")]
#[get("/mixnodes/rewarded/detailed")]
pub async fn get_rewarded_set_detailed(
    cache: &State<ValidatorCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(cache.rewarded_set_detailed().await.value)
}

#[openapi(tag = "contract-cache")]
#[get("/mixnodes/active")]
pub async fn get_active_set(cache: &State<ValidatorCache>) -> Json<Vec<MixNodeDetails>> {
    Json(cache.active_set().await)
}

#[openapi(tag = "contract-cache")]
#[get("/mixnodes/active/detailed")]
pub async fn get_active_set_detailed(
    cache: &State<ValidatorCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(cache.active_set_detailed().await.value)
}

#[openapi(tag = "contract-cache")]
#[get("/mixnodes/blacklisted")]
pub async fn get_blacklisted_mixnodes(
    cache: &State<ValidatorCache>,
) -> Json<Option<HashSet<MixId>>> {
    Json(cache.mixnodes_blacklist().await.map(|c| c.value))
}

#[openapi(tag = "contract-cache")]
#[get("/gateways/blacklisted")]
pub async fn get_blacklisted_gateways(
    cache: &State<ValidatorCache>,
) -> Json<Option<HashSet<String>>> {
    Json(cache.gateways_blacklist().await.map(|c| c.value))
}

#[openapi(tag = "contract-cache")]
#[get("/epoch/reward_params")]
pub async fn get_interval_reward_params(
    cache: &State<ValidatorCache>,
) -> Json<Option<RewardingParams>> {
    Json(cache.interval_reward_params().await.value)
}

#[openapi(tag = "contract-cache")]
#[get("/epoch/current")]
pub async fn get_current_epoch(cache: &State<ValidatorCache>) -> Json<Option<Interval>> {
    Json(cache.current_interval().await.value)
}
