// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::api_routes::helpers::EpochIdParam;
use crate::ecash::error::EcashError;
use crate::ecash::helpers::blind_sign;
use crate::ecash::state::EcashState;
use crate::ecash::storage::models::DepositUsage;
use crate::node_status_api::models::AxumResult;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::{Json, Router};
use nym_api_requests::ecash::{
    BlindSignRequestBody, BlindedSignatureResponse, PartialCoinIndicesSignatureResponse,
    PartialExpirationDateSignatureResponse,
};
use nym_coconut_dkg_common::types::EpochId;
use nym_ecash_time::{cred_exp_date, EcashTime};
use nym_http_api_common::{FormattedResponse, Output, OutputParams};
use nym_validator_client::nym_api::RFC_3339_DATE_FORMAT;
use serde::Deserialize;
use std::ops::Deref;
use std::sync::Arc;
use time::Date;
use tracing::{debug, trace};
use utoipa::IntoParams;

pub(crate) fn partial_signing_routes() -> Router<AppState> {
    Router::new()
        .route("/blind-sign", axum::routing::post(post_blind_sign))
        .route(
            "/partial-expiration-date-signatures",
            axum::routing::get(partial_expiration_date_signatures),
        )
        .route(
            "/partial-coin-indices-signatures",
            axum::routing::get(partial_coin_indices_signatures),
        )
}

#[utoipa::path(
    tag = "Ecash",
    post,
    request_body = BlindSignRequestBody,
    path = "/v1/ecash/blind-sign",
    responses(
         (status = 200, content(
            (BlindedSignatureResponse = "application/json"),
            (BlindedSignatureResponse = "application/yaml"),
            (BlindedSignatureResponse = "application/bincode")
        )),
        (status = 400, body = String, description = "this nym-api is not an ecash signer in the current epoch"),
    ),
    params(OutputParams)
)]
async fn post_blind_sign(
    Query(output): Query<OutputParams>,
    State(state): State<Arc<EcashState>>,
    Json(blind_sign_request_body): Json<BlindSignRequestBody>,
) -> AxumResult<FormattedResponse<BlindedSignatureResponse>> {
    state.ensure_signer().await?;
    let output = output.output.unwrap_or_default();

    debug!("Received blind sign request");
    trace!("body: {:?}", blind_sign_request_body);

    // check if we have the signing key available
    debug!("checking if we actually have ecash keys derived...");
    let signing_key = state.ecash_signing_key().await?;

    // basic check of expiration date validity
    if blind_sign_request_body.expiration_date > cred_exp_date().ecash_date() {
        return Err(EcashError::ExpirationDateTooLate.into());
    }

    // see if we're not in the middle of new dkg
    state.ensure_dkg_not_in_progress().await?;

    // check if we already issued a credential for this deposit
    let deposit_id = blind_sign_request_body.deposit_id;
    debug!(
        "checking if we have already issued credential for this deposit (deposit_id: {deposit_id})",
    );
    match state.deposit_usage(deposit_id).await? {
        // a repeated request for a deposit whose share we still hold gets that same share back
        DepositUsage::Issued(blinded_signature) => {
            return Ok(output.to_response(BlindedSignatureResponse { blinded_signature }))
        }

        // we no longer hold the share, but the deposit was still spent on it. signing again here
        // would mint a second ticketbook against a single payment.
        DepositUsage::Pruned => return Err(EcashError::DepositAlreadyUsed { deposit_id }.into()),

        DepositUsage::Unused => (),
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
    let blinded_signature = blind_sign(&blind_sign_request_body, signing_key.deref())?;

    // store the information locally
    debug!("storing the issued credential in the database");
    state
        .store_issued_ticketbook(blind_sign_request_body, &blinded_signature)
        .await?;

    // finally return the credential to the client
    Ok(output.to_response(BlindedSignatureResponse { blinded_signature }))
}

#[derive(Deserialize, IntoParams)]
struct ExpirationDateParam {
    expiration_date: Option<String>,
    epoch_id: Option<EpochId>,
    output: Option<Output>,
}

#[utoipa::path(
    tag = "Ecash",
    get,
    params(
        ExpirationDateParam
    ),
    path = "/v1/ecash/partial-expiration-date-signatures",
    responses(
        (status = 200, content(
            (PartialExpirationDateSignatureResponse = "application/json"),
            (PartialExpirationDateSignatureResponse = "application/yaml"),
            (PartialExpirationDateSignatureResponse = "application/bincode")
        )),
        (status = 400, body = String, description = "this nym-api is not an ecash signer in the requested epoch, or that epoch's DKG ceremony has not concluded"),
    )
)]
async fn partial_expiration_date_signatures(
    State(state): State<Arc<EcashState>>,
    Query(ExpirationDateParam {
        expiration_date,
        epoch_id,
        output,
    }): Query<ExpirationDateParam>,
) -> AxumResult<FormattedResponse<PartialExpirationDateSignatureResponse>> {
    let output = output.unwrap_or_default();

    let expiration_date = match expiration_date {
        None => cred_exp_date().ecash_date(),
        Some(raw) => Date::parse(&raw, RFC_3339_DATE_FORMAT)
            .map_err(|_| EcashError::MalformedExpirationDate { raw })?,
    };

    let epoch_id = match epoch_id {
        Some(epoch_id) => epoch_id,
        None => state.current_dkg_epoch().await?,
    };

    // the caller wants this epoch's material, so it's this epoch's signers that have to answer
    state.ensure_signer_for_epoch(epoch_id).await?;

    // an aggregator collecting a past epoch's signatures depends on us answering this while a
    // later ceremony runs, so only that ceremony's own epoch is refused
    state.ensure_ceremony_concluded(epoch_id).await?;

    let expiration_date_signatures = state
        .partial_expiration_date_signatures(expiration_date, epoch_id)
        .await?;

    Ok(output.to_response(PartialExpirationDateSignatureResponse {
        epoch_id: expiration_date_signatures.epoch_id,
        expiration_date,
        signatures: expiration_date_signatures.signatures.clone(),
    }))
}

#[utoipa::path(
    tag = "Ecash",
    get,
    params(
        EpochIdParam
    ),
    path = "/v1/ecash/partial-coin-indices-signatures",
    responses(
        (status = 200, content(
            (PartialCoinIndicesSignatureResponse = "application/json"),
            (PartialCoinIndicesSignatureResponse = "application/yaml"),
            (PartialCoinIndicesSignatureResponse = "application/bincode")
        )),
        (status = 400, body = String, description = "this nym-api is not an ecash signer in the requested epoch, or that epoch's DKG ceremony has not concluded"),
    )
)]
async fn partial_coin_indices_signatures(
    State(state): State<Arc<EcashState>>,
    Query(EpochIdParam { epoch_id, output }): Query<EpochIdParam>,
) -> AxumResult<FormattedResponse<PartialCoinIndicesSignatureResponse>> {
    let epoch_id = match epoch_id {
        Some(epoch_id) => epoch_id,
        None => state.current_dkg_epoch().await?,
    };

    // the caller wants this epoch's material, so it's this epoch's signers that have to answer
    state.ensure_signer_for_epoch(epoch_id).await?;

    // as above: refuse only the epoch whose ceremony is still running
    state.ensure_ceremony_concluded(epoch_id).await?;

    let coin_indices_signatures = state.partial_coin_index_signatures(Some(epoch_id)).await?;

    Ok(output
        .unwrap_or_default()
        .to_response(PartialCoinIndicesSignatureResponse {
            epoch_id: coin_indices_signatures.epoch_id,
            signatures: coin_indices_signatures.signatures.clone(),
        }))
}
