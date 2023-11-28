// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::circulating_supply_api::cache::CirculatingSupplyCache;
use crate::node_status_api::models::ErrorResponse;
use nym_api_requests::models::CirculatingSupplyResponse;
use nym_validator_client::nyxd::Coin;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;

// TODO: this is not the best place to put it, it should be more centralised,
// but for a quick fix, that's good enough for now...
// (for proper solution we should be managing `NymNetworkDetails` via rocket and grabbing display exponent
// value from the mix denom here.
const UNYM_RATIO: f64 = 1000000.;

fn unym_coin_to_float_unym(coin: Coin) -> f64 {
    // our total supply can't exceed 1B so an overflow here is impossible
    // (if it happened, then we SHOULD crash)
    coin.amount as f64 / UNYM_RATIO
}

#[openapi(tag = "circulating-supply")]
#[get("/circulating-supply")]
pub(crate) async fn get_full_circulating_supply(
    cache: &State<CirculatingSupplyCache>,
) -> Result<Json<CirculatingSupplyResponse>, ErrorResponse> {
    match cache.get_circulating_supply().await {
        Some(value) => Ok(Json(value)),
        None => Err(ErrorResponse::new(
            "unavailable",
            Status::InternalServerError,
        )),
    }
}

#[openapi(tag = "circulating-supply")]
#[get("/circulating-supply/total-supply-value")]
pub(crate) async fn get_total_supply(
    cache: &State<CirculatingSupplyCache>,
) -> Result<Json<f64>, ErrorResponse> {
    let full_circulating_supply = match cache.get_circulating_supply().await {
        Some(res) => res,
        None => {
            return Err(ErrorResponse::new(
                "unavailable",
                Status::InternalServerError,
            ))
        }
    };

    Ok(Json(unym_coin_to_float_unym(
        full_circulating_supply.total_supply.into(),
    )))
}

#[openapi(tag = "circulating-supply")]
#[get("/circulating-supply/circulating-supply-value")]
pub(crate) async fn get_circulating_supply(
    cache: &State<CirculatingSupplyCache>,
) -> Result<Json<f64>, ErrorResponse> {
    let full_circulating_supply = match cache.get_circulating_supply().await {
        Some(res) => res,
        None => {
            return Err(ErrorResponse::new(
                "unavailable",
                Status::InternalServerError,
            ))
        }
    };

    Ok(Json(unym_coin_to_float_unym(
        full_circulating_supply.circulating_supply.into(),
    )))
}
