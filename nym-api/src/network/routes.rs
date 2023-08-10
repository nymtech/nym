// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network::models::NetworkDetails;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use std::ops::Deref;

#[openapi(tag = "network")]
#[get("/details")]
pub(crate) fn network_details(details: &State<NetworkDetails>) -> Json<NetworkDetails> {
    Json(details.deref().clone())
}
