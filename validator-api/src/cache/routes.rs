// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cache::ValidatorCache;
use mixnet_contract_common::{GatewayBond, MixNodeBond};
use rocket::serde::json::Json;
use rocket::State;

#[get("/mixnodes")]
pub(crate) async fn get_mixnodes(cache: &State<ValidatorCache>) -> Json<Vec<MixNodeBond>> {
    Json(cache.mixnodes().await.value)
}

#[get("/gateways")]
pub(crate) async fn get_gateways(cache: &State<ValidatorCache>) -> Json<Vec<GatewayBond>> {
    Json(cache.gateways().await.value)
}

#[get("/mixnodes/rewarded")]
pub(crate) async fn get_rewarded_mixnodes(cache: &State<ValidatorCache>) -> Json<Vec<MixNodeBond>> {
    Json(cache.rewarded_mixnodes().await.value)
}

#[get("/mixnodes/active")]
pub(crate) async fn get_active_mixnodes(cache: &State<ValidatorCache>) -> Json<Vec<MixNodeBond>> {
    Json(cache.active_mixnodes().await.value)
}
