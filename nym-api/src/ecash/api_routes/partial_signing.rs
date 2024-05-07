// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::EcashError;
use crate::ecash::helpers::blind_sign;
use crate::ecash::state::EcashState;
use nym_api_requests::ecash::{
    BlindSignRequestBody, BlindedSignatureResponse, PartialCoinIndicesSignatureResponse,
    PartialExpirationDateSignatureResponse,
};
use nym_ecash_time::{cred_exp_date, EcashTime};
use nym_validator_client::nym_api::rfc_3339_date;
use rocket::serde::json::Json;
use rocket::State as RocketState;
use rocket_okapi::openapi;
use std::ops::Deref;
use time::Date;

#[openapi(tag = "Ecash")]
#[post("/blind-sign", data = "<blind_sign_request_body>")]
pub async fn post_blind_sign(
    blind_sign_request_body: Json<BlindSignRequestBody>,
    state: &RocketState<EcashState>,
) -> crate::ecash::error::Result<Json<BlindedSignatureResponse>> {
    debug!("Received blind sign request");
    trace!("body: {:?}", blind_sign_request_body);

    // check if we have the signing key available
    debug!("checking if we actually have ecash keys derived...");
    let signing_key = state.ecash_signing_key().await?;

    // basic check of expiration date validity
    if blind_sign_request_body.expiration_date > cred_exp_date().ecash_date() {
        return Err(EcashError::ExpirationDateTooLate);
    }

    // see if we're not in the middle of new dkg
    state.ensure_dkg_not_in_progress().await?;

    // check if we already issued a credential for this deposit
    let deposit_id = blind_sign_request_body.deposit_id;
    debug!(
        "checking if we have already issued credential for this deposit (deposit_id: {deposit_id})",
    );
    if let Some(blinded_signature) = state.already_issued(deposit_id).await? {
        return Ok(Json(BlindedSignatureResponse { blinded_signature }));
    }

    //check if account was blacklisted
    let pub_key_bs58 = blind_sign_request_body.ecash_pubkey.to_base58_string();
    state.aux.ensure_not_blacklisted(&pub_key_bs58).await?;

    // get the deposit details of the claimed id
    debug!("getting deposit details from the chain");
    let deposit = state.get_deposit(deposit_id).await?;

    // check validity of the request
    debug!("fully validating received request");
    state
        .validate_request(&blind_sign_request_body, deposit)
        .await?;

    // produce the partial signature
    debug!("producing the partial credential");
    let blinded_signature = blind_sign(blind_sign_request_body.deref(), signing_key.deref())?;

    // store the information locally
    debug!("storing the issued credential in the database");
    state
        .store_issued_credential(blind_sign_request_body.into_inner(), &blinded_signature)
        .await?;

    // finally return the credential to the client
    Ok(Json(BlindedSignatureResponse { blinded_signature }))
}

#[openapi(tag = "Ecash")]
#[get("/partial-expiration-date-signatures?<expiration_date>")]
pub async fn partial_expiration_date_signatures(
    expiration_date: Option<String>,
    state: &RocketState<EcashState>,
) -> crate::ecash::error::Result<Json<PartialExpirationDateSignatureResponse>> {
    let expiration_date = match expiration_date {
        None => cred_exp_date().ecash_date(),
        Some(raw) => Date::parse(&raw, &rfc_3339_date())
            .map_err(|_| EcashError::MalformedExpirationDate { raw })?,
    };

    // see if we're not in the middle of new dkg
    state.ensure_dkg_not_in_progress().await?;

    let expiration_date_signatures = state
        .partial_expiration_date_signatures(expiration_date)
        .await?;

    Ok(Json(PartialExpirationDateSignatureResponse {
        epoch_id: expiration_date_signatures.epoch_id,
        expiration_date,
        signatures: expiration_date_signatures.signatures.clone(),
    }))
}

#[openapi(tag = "Ecash")]
#[get("/partial-coin-indices-signatures?<epoch_id>")]
pub async fn partial_coin_indices_signatures(
    epoch_id: Option<u64>,
    state: &RocketState<EcashState>,
) -> crate::ecash::error::Result<Json<PartialCoinIndicesSignatureResponse>> {
    // see if we're not in the middle of new dkg
    state.ensure_dkg_not_in_progress().await?;

    let coin_indices_signatures = state.partial_coin_index_signatures(epoch_id).await?;

    Ok(Json(PartialCoinIndicesSignatureResponse {
        epoch_id: coin_indices_signatures.epoch_id,
        signatures: coin_indices_signatures.signatures.clone(),
    }))
}
