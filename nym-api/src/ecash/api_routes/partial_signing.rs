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
use nym_http_api_common::{FormattedResponse, Output};
use nym_validator_client::nym_api::RFC_3339_DATE_FORMAT;
use serde::Deserialize;
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
        (status = 400, body = String, description = "\
            the requested epoch is not one this nym-api will issue under - either its ceremony has \
            not concluded or it has been superseded; or no epoch was requested while a ceremony \
            concluded recently, so which one is meant has to be stated; or this nym-api is not an \
            ecash signer for that epoch, or holds no key for it; or this deposit's ticketbook was \
            already issued under a different epoch, or was issued and its data has since been \
            pruned. None of these consume the deposit."),
    ),
    params(EpochIdParam)
)]
async fn post_blind_sign(
    Query(EpochIdParam { epoch_id, output }): Query<EpochIdParam>,
    State(state): State<Arc<EcashState>>,
    Json(blind_sign_request_body): Json<BlindSignRequestBody>,
) -> AxumResult<FormattedResponse<BlindedSignatureResponse>> {
    let output = output.unwrap_or_default();

    debug!("Received blind sign request");
    trace!("body: {:?}", blind_sign_request_body);

    // which epoch we would put a signature under right now. this replaces the blanket refusal
    // while a ceremony runs: the epoch under ceremony has no keys, but the one it is replacing
    // does, and its credentials are still being spent.
    let issuable = state.issuable_epochs().await?;

    // basic check of expiration date validity
    if blind_sign_request_body.expiration_date > cred_exp_date().ecash_date() {
        return Err(EcashError::ExpirationDateTooLate.into());
    }

    // check if we already issued a credential for this deposit
    let deposit_id = blind_sign_request_body.deposit_id;
    debug!(
        "checking if we have already issued credential for this deposit (deposit_id: {deposit_id})",
    );
    match state.deposit_usage(deposit_id).await? {
        DepositUsage::Issued {
            share,
            issued_for_epoch,
        } => {
            state.ensure_signer_for_epoch(issued_for_epoch).await?;

            // handing back a share we already hold consumes nothing, so there is no epoch to
            // choose and nothing to be ambiguous about: it is enough that this is the epoch the
            // caller named. A caller that named none gets it if we would still sign for it.
            //
            // note this deliberately serves an epoch we would no longer *issue* under, as long as
            // it was asked for: that is how a collection stranded across a rotation is recovered.
            let acceptable = match epoch_id {
                Some(requested) => issued_for_epoch == requested,
                None => issuable.accepts(issued_for_epoch),
            };

            if !acceptable {
                // a share is bound to the key that signed it. one signed under a different epoch
                // cannot be unblinded against the epoch being collected, so the caller drops it
                // and falls short of the threshold - every time, since this is a cache. refusing
                // at least names the epoch under which the share can still be claimed.
                return Err(EcashError::IssuedUnderDifferentEpoch {
                    deposit_id,
                    issued_for_epoch,
                    requested_epoch: epoch_id.unwrap_or(issuable.issuable),
                }
                .into());
            }

            return Ok(output.to_response(BlindedSignatureResponse {
                blinded_signature: share,
            }));
        }

        // we no longer hold the share, but the deposit was still spent on it. signing again here
        // would mint a second ticketbook against a single payment.
        DepositUsage::Pruned => return Err(EcashError::DepositAlreadyUsed { deposit_id }.into()),

        DepositUsage::Unused => (),
    }

    // a fresh deposit, so an epoch has to be settled on and signed under. everything from here to
    // the signature refuses without consuming the deposit.
    let issuing_epoch = match epoch_id {
        Some(requested) => requested,
        None => issuable.default_for_fresh_issuance()?,
    };
    issuable.ensure_issuable(issuing_epoch)?;

    // it is that epoch's signer set that answers for it: an api that has since joined or left the
    // group is not the authority on an epoch it was not part of
    state.ensure_signer_for_epoch(issuing_epoch).await?;

    debug!("checking if we actually have ecash keys derived for epoch {issuing_epoch}...");
    let issuance_keys = state.ecash_issuance_keys(issuing_epoch).await?;

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
    let blinded_signature = blind_sign(&blind_sign_request_body, issuance_keys.signing_key())?;

    // store the information locally, against the epoch of the key that just signed it. the lookup
    // above guarantees these agree, which is what the epoch-aware cache relies on.
    debug!("storing the issued credential in the database");
    state
        .store_issued_ticketbook(
            blind_sign_request_body,
            &blinded_signature,
            issuance_keys.issued_for_epoch,
        )
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

    let epoch_id = state.requested_epoch(epoch_id).await?;

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
    let epoch_id = state.requested_epoch(epoch_id).await?;

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
