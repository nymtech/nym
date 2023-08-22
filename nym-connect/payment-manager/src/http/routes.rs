// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use crate::state::State;
use nym_payment_manager_common::PaymentResponse;
use rocket::serde::json::Json;
use rocket::{post, State as RocketState};
use rocket_okapi::okapi::schemars;
use rocket_okapi::openapi;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

//  All strings are base58 encoded representations of structs
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ClaimPaymentRequestBody {
    pub serial_number: String,
    pub deposit_address: String,
}

#[openapi(tag = "payment")]
#[post("/claim_payment", data = "<claim_payment_request_body>")]
pub async fn claim_payment(
    claim_payment_request_body: Json<ClaimPaymentRequestBody>,
    state: &RocketState<State>,
) -> Result<Json<PaymentResponse>, Error> {
    let payment = state
        .storage
        .manager
        .get_payment(&claim_payment_request_body.serial_number)
        .await?;

    Ok(Json(PaymentResponse {
        unyms_bought: payment.unyms_bought as u64,
    }))
}
