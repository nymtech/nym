// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cache::{Cache, ValidatorCache};
use mixnet_contract::{GatewayBond, MixNodeBond};
use rocket::serde::json::Json;
use rocket::State;

#[get("/mixnodes")]
pub(crate) async fn get_mixnodes(cache: &State<ValidatorCache>) -> Json<Cache<Vec<MixNodeBond>>> {
    Json(cache.mixnodes().await)
}

#[get("/gateways")]
pub(crate) async fn get_gateways(cache: &State<ValidatorCache>) -> Json<Cache<Vec<GatewayBond>>> {
    Json(cache.gateways().await)
}
