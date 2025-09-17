// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::api_routes::helpers::EpochIdParam;
use crate::ecash::error::EcashError;
use crate::ecash::state::EcashState;
use crate::node_status_api::models::AxumResult;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::Router;
use nym_api_requests::ecash::models::{
    AggregatedCoinIndicesSignatureResponse, AggregatedExpirationDateSignatureResponse,
};
use nym_api_requests::ecash::VerificationKeyResponse;
use nym_coconut_dkg_common::types::EpochId;
use nym_ecash_time::{cred_exp_date, EcashTime};
use nym_http_api_common::{FormattedResponse, Output};
use nym_validator_client::nym_api::rfc_3339_date;
use serde::Deserialize;
use std::sync::Arc;
use time::Date;
use tracing::trace;
use utoipa::IntoParams;

/// routes with globally aggregated keys, signatures, etc.
pub(crate) fn aggregation_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/master-verification-key",
            axum::routing::get(master_verification_key),
        )
        .route(
            "/aggregated-expiration-date-signatures",
            axum::routing::get(expiration_date_signatures),
        )
        .route(
            "/aggregated-coin-indices-signatures",
            axum::routing::get(coin_indices_signatures),
        )
}

#[utoipa::path(
    tag = "Ecash Global Data",
    get,
    params(
        EpochIdParam
    ),
    path = "/v1/ecash/master-verification-key",
    responses(
        (status = 200, content(
            (VerificationKeyResponse = "application/json"),
            (VerificationKeyResponse = "application/yaml"),
            (VerificationKeyResponse = "application/bincode")
        ))
    ),
)]
async fn master_verification_key(
    State(state): State<Arc<EcashState>>,
    Query(EpochIdParam { epoch_id, output }): Query<EpochIdParam>,
) -> AxumResult<FormattedResponse<VerificationKeyResponse>> {
    trace!("aggregated_verification_key request");
    let output = output.unwrap_or_default();

    // see if we're not in the middle of new dkg
    state.ensure_dkg_not_in_progress().await?;

    let key = state.master_verification_key(epoch_id).await?;

    Ok(output.to_response(VerificationKeyResponse::new(key.clone())))
}

#[derive(Deserialize, IntoParams)]
struct ExpirationDateParam {
    expiration_date: Option<String>,
    epoch_id: Option<EpochId>,
    output: Option<Output>,
}

#[utoipa::path(
    tag = "Ecash Global Data",
    get,
    params(
        ExpirationDateParam
    ),
    path = "/v1/ecash/aggregated-expiration-date-signatures",
    responses(
        (status = 200, content(
            (AggregatedExpirationDateSignatureResponse = "application/json"),
            (AggregatedExpirationDateSignatureResponse = "application/yaml"),
            (AggregatedExpirationDateSignatureResponse = "application/bincode")
        ))
    ),
)]
async fn expiration_date_signatures(
    State(state): State<Arc<EcashState>>,
    Query(ExpirationDateParam {
        expiration_date,
        epoch_id,
        output,
    }): Query<ExpirationDateParam>,
) -> AxumResult<FormattedResponse<AggregatedExpirationDateSignatureResponse>> {
    trace!("aggregated_expiration_date_signatures request");
    let output = output.unwrap_or_default();

    let expiration_date = match expiration_date {
        None => cred_exp_date().ecash_date(),
        Some(raw) => Date::parse(&raw, &rfc_3339_date())
            .map_err(|_| EcashError::MalformedExpirationDate { raw })?,
    };

    // see if we're not in the middle of new dkg
    state.ensure_dkg_not_in_progress().await?;

    let epoch_id = match epoch_id {
        Some(epoch_id) => epoch_id,
        None => state.current_dkg_epoch().await?,
    };

    let expiration_date_signatures = state
        .master_expiration_date_signatures(expiration_date, epoch_id)
        .await?;

    Ok(
        output.to_response(AggregatedExpirationDateSignatureResponse {
            epoch_id: expiration_date_signatures.epoch_id,
            expiration_date,
            signatures: expiration_date_signatures.signatures.clone(),
        }),
    )
}

#[utoipa::path(
    tag = "Ecash Global Data",
    get,
    params(
        EpochIdParam
    ),
    path = "/v1/ecash/aggregated-coin-indices-signatures",
    responses(
        (status = 200, content(
            (AggregatedCoinIndicesSignatureResponse = "application/json"),
            (AggregatedCoinIndicesSignatureResponse = "application/yaml"),
            (AggregatedCoinIndicesSignatureResponse = "application/bincode")
        ))
    ),
)]
async fn coin_indices_signatures(
    Query(EpochIdParam { epoch_id, output }): Query<EpochIdParam>,
    State(state): State<Arc<EcashState>>,
) -> AxumResult<FormattedResponse<AggregatedCoinIndicesSignatureResponse>> {
    trace!("aggregated_coin_indices_signatures request");

    let output = output.unwrap_or_default();
    // see if we're not in the middle of new dkg
    state.ensure_dkg_not_in_progress().await?;

    let coin_indices_signatures = state.master_coin_index_signatures(epoch_id).await?;

    Ok(output.to_response(AggregatedCoinIndicesSignatureResponse {
        epoch_id: coin_indices_signatures.epoch_id,
        signatures: coin_indices_signatures.signatures.clone(),
    }))
}
