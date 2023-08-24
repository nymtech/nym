// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use crate::state::State;
use nym_payment_manager_common::PaymentResponse;
use nym_validator_client::nyxd::{AccountId, Coin};
use rocket::serde::json::Json;
use rocket::{post, State as RocketState};
use rocket_okapi::okapi::schemars;
use rocket_okapi::openapi;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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
        .await?
        .ok_or(Error::InvalidPaymentRequest)?;

    let recipient = AccountId::from_str(&claim_payment_request_body.deposit_address)
        .map_err(|_| Error::BadAddress)?;
    let amount = vec![Coin::new(
        payment.unyms_bought as u128,
        state.config.denom(),
    )];
    state
        .client
        .0
        .read()
        .await
        .send(&recipient, amount, "deposit via payment", None)
        .await?;

    state.storage.manager.update_payment(payment.id).await?;

    Ok(Json(PaymentResponse {
        unyms_bought: payment.unyms_bought as u64,
    }))
}
