// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::EcashError;
use crate::ecash::state::EcashState;
use crate::node_status_api::models::AxumResult;
use crate::v2::AxumAppState;
use axum::extract::Path;
use axum::{Json, Router};
use log::trace;
use nym_api_requests::ecash::models::{
    AggregatedCoinIndicesSignatureResponse, AggregatedExpirationDateSignatureResponse,
};
use nym_api_requests::ecash::VerificationKeyResponse;
use nym_ecash_time::{cred_exp_date, EcashTime};
use nym_validator_client::nym_api::rfc_3339_date;
use std::sync::Arc;
use time::Date;

/// routes with globally aggregated keys, signatures, etc.
pub(crate) fn aggregation_routes(ecash_state: Arc<EcashState>) -> Router<AxumAppState> {
    Router::new()
        .route(
            "/master-verification-key:epoch_id",
            axum::routing::get({
                let ecash_state = Arc::clone(&ecash_state);
                |epoch_id| master_verification_key(epoch_id, ecash_state)
            }),
        )
        .route(
            "/aggregated-expiration-date-signatures:expiration_date",
            axum::routing::get({
                let ecash_state = Arc::clone(&ecash_state);
                |expiration_date| expiration_date_signatures(expiration_date, ecash_state)
            }),
        )
        .route(
            "/aggregated-coin-indices-signatures:epoch_id",
            axum::routing::get({
                let ecash_state = Arc::clone(&ecash_state);
                |epoch_id| coin_indices_signatures(epoch_id, ecash_state)
            }),
        )
}

// TODO dz swagger annotate
// #[openapi(tag = "Ecash Global Data")]
// #[get("/master-verification-key?<epoch_id>")]
async fn master_verification_key(
    Path(epoch_id): Path<Option<u64>>,
    state: Arc<EcashState>,
) -> AxumResult<Json<VerificationKeyResponse>> {
    trace!("aggregated_verification_key request");

    // see if we're not in the middle of new dkg
    state.ensure_dkg_not_in_progress().await?;

    let key = state.master_verification_key(epoch_id).await?;

    Ok(Json(VerificationKeyResponse::new(key.clone())))
}

// #[openapi(tag = "Ecash Global Data")]
// #[get("/aggregated-expiration-date-signatures?<expiration_date>")]
async fn expiration_date_signatures(
    Path(expiration_date): Path<Option<String>>,
    state: Arc<EcashState>,
) -> AxumResult<Json<AggregatedExpirationDateSignatureResponse>> {
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

// #[openapi(tag = "Ecash Global Data")]
// #[get("/aggregated-coin-indices-signatures?<epoch_id>")]
async fn coin_indices_signatures(
    Path(epoch_id): Path<Option<u64>>,
    state: Arc<EcashState>,
) -> AxumResult<Json<AggregatedCoinIndicesSignatureResponse>> {
    trace!("aggregated_coin_indices_signatures request");

    // see if we're not in the middle of new dkg
    state.ensure_dkg_not_in_progress().await?;

    let coin_indices_signatures = state.master_coin_index_signatures(epoch_id).await?;

    Ok(Json(AggregatedCoinIndicesSignatureResponse {
        epoch_id: coin_indices_signatures.epoch_id,
        signatures: coin_indices_signatures.signatures.clone(),
    }))
}
