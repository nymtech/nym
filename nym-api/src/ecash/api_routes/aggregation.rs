// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::{EcashError, Result};
use crate::ecash::state::EcashState;
use log::trace;
use nym_api_requests::ecash::models::{
    AggregatedCoinIndicesSignatureResponse, AggregatedExpirationDateSignatureResponse,
};
use nym_api_requests::ecash::VerificationKeyResponse;
use nym_ecash_time::{cred_exp_date, EcashTime};
use nym_validator_client::nym_api::rfc_3339_date;
use rocket::serde::json::Json;
use rocket::State as RocketState;
use rocket_okapi::openapi;
use time::Date;

// routes with globally aggregated keys, signatures, etc.

#[openapi(tag = "Ecash Global Data")]
#[get("/master-verification-key?<epoch_id>")]
pub async fn master_verification_key(
    epoch_id: Option<u64>,
    state: &RocketState<EcashState>,
) -> Result<Json<VerificationKeyResponse>> {
    trace!("aggregated_verification_key request");

    // see if we're not in the middle of new dkg
    state.ensure_dkg_not_in_progress().await?;

    let key = state.master_verification_key(epoch_id).await?;

    Ok(Json(VerificationKeyResponse::new(key.clone())))
}

#[openapi(tag = "Ecash Global Data")]
#[get("/aggregated-expiration-date-signatures?<expiration_date>")]
pub async fn expiration_date_signatures(
    expiration_date: Option<String>,
    state: &RocketState<EcashState>,
) -> Result<Json<AggregatedExpirationDateSignatureResponse>> {
    trace!("aggregated_expiration_date_signatures request");

    let expiration_date = match expiration_date {
        None => cred_exp_date().ecash_date(),
        Some(raw) => Date::parse(&raw, &rfc_3339_date())
            .map_err(|_| EcashError::MalformedExpirationDate { raw })?,
    };

    // see if we're not in the middle of new dkg
    state.ensure_dkg_not_in_progress().await?;

    let expiration_date_signatures = state
        .master_expiration_date_signatures(expiration_date)
        .await?;

    Ok(Json(AggregatedExpirationDateSignatureResponse {
        epoch_id: expiration_date_signatures.epoch_id,
        expiration_date,
        signatures: expiration_date_signatures.signatures.clone(),
    }))
}

#[openapi(tag = "Ecash Global Data")]
#[get("/aggregated-coin-indices-signatures?<epoch_id>")]
pub async fn coin_indices_signatures(
    epoch_id: Option<u64>,
    state: &RocketState<EcashState>,
) -> Result<Json<AggregatedCoinIndicesSignatureResponse>> {
    trace!("aggregated_coin_indices_signatures request");

    // see if we're not in the middle of new dkg
    state.ensure_dkg_not_in_progress().await?;

    let coin_indices_signatures = state.master_coin_index_signatures(epoch_id).await?;

    Ok(Json(AggregatedCoinIndicesSignatureResponse {
        epoch_id: coin_indices_signatures.epoch_id,
        signatures: coin_indices_signatures.signatures.clone(),
    }))
}
