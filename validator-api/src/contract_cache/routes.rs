// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract_cache::ValidatorCache;
use mixnet_contract_common::reward_params::EpochRewardParams;
use mixnet_contract_common::{GatewayBond, Interval, MixNodeBond};
use rocket::serde::json::Json;
use rocket::State;
use std::collections::HashSet;

#[get("/mixnodes")]
pub async fn get_mixnodes(cache: &State<ValidatorCache>) -> Json<Vec<MixNodeBond>> {
    Json(cache.mixnodes().await)
}

#[get("/gateways")]
pub async fn get_gateways(cache: &State<ValidatorCache>) -> Json<Vec<GatewayBond>> {
    Json(cache.gateways().await)
}

#[get("/mixnodes/rewarded")]
pub async fn get_rewarded_set(cache: &State<ValidatorCache>) -> Json<Vec<MixNodeBond>> {
    Json(cache.rewarded_set().await.value)
}

#[get("/mixnodes/active")]
pub async fn get_active_set(cache: &State<ValidatorCache>) -> Json<Vec<MixNodeBond>> {
    Json(cache.active_set().await.value)
}

#[get("/mixnodes/blacklisted")]
pub async fn get_blacklisted_mixnodes(
    cache: &State<ValidatorCache>,
) -> Json<Option<HashSet<String>>> {
    Json(cache.mixnodes_blacklist().await.map(|c| c.value))
}

#[get("/gateways/blacklisted")]
pub async fn get_blacklisted_gateways(
    cache: &State<ValidatorCache>,
) -> Json<Option<HashSet<String>>> {
    Json(cache.gateways_blacklist().await.map(|c| c.value))
}

#[get("/epoch/reward_params")]
pub async fn get_epoch_reward_params(cache: &State<ValidatorCache>) -> Json<EpochRewardParams> {
    Json(cache.epoch_reward_params().await.value)
}

#[get("/epoch/current")]
pub async fn get_current_epoch(cache: &State<ValidatorCache>) -> Json<Option<Interval>> {
    Json(cache.current_epoch().await.value)
}
