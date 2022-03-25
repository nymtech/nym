// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract_cache::ValidatorCache;
use mixnet_contract_common::{GatewayBond, MixNodeBond};
use rocket::serde::json::Json;
use rocket::State;
use std::collections::HashSet;

#[get("/mixnodes")]
pub(crate) async fn get_mixnodes(cache: &State<ValidatorCache>) -> Json<Vec<MixNodeBond>> {
    Json(cache.mixnodes().await)
}

#[get("/gateways")]
pub(crate) async fn get_gateways(cache: &State<ValidatorCache>) -> Json<Vec<GatewayBond>> {
    Json(cache.gateways().await)
}

#[get("/mixnodes/rewarded")]
pub(crate) async fn get_rewarded_set(cache: &State<ValidatorCache>) -> Json<Vec<MixNodeBond>> {
    Json(cache.rewarded_set().await.value)
}

#[get("/mixnodes/active")]
pub(crate) async fn get_active_set(cache: &State<ValidatorCache>) -> Json<Vec<MixNodeBond>> {
    Json(cache.active_set().await.value)
}

#[get("/mixnodes/blacklisted")]
pub(crate) async fn get_blacklisted_mixnodes(
    cache: &State<ValidatorCache>,
) -> Json<HashSet<String>> {
    Json(cache.mixnodes_blacklist().await.value)
}

#[get("/gateways/blacklisted")]
pub(crate) async fn get_blacklisted_gateways(
    cache: &State<ValidatorCache>,
) -> Json<HashSet<String>> {
    Json(cache.gateways_blacklist().await.value)
}
