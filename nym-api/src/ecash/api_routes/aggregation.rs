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
use nym_validator_client::nym_api::RFC_3339_DATE_FORMAT;
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
        )),
        (status = 400, body = String, description = "the requested epoch's DKG ceremony has not concluded, so it has no key yet"),
    ),
)]
async fn master_verification_key(
    State(state): State<Arc<EcashState>>,
    Query(EpochIdParam { epoch_id, output }): Query<EpochIdParam>,
) -> AxumResult<FormattedResponse<VerificationKeyResponse>> {
    trace!("aggregated_verification_key request");
    let output = output.unwrap_or_default();

    let epoch_id = state.requested_epoch(epoch_id).await?;

    // a concluded epoch's key is fixed, so a ceremony running for some *other* epoch is no
    // reason to withhold it
    state.ensure_ceremony_concluded(epoch_id).await?;

    let key = state.master_verification_key(Some(epoch_id)).await?;

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
        )),
        (status = 400, body = String, description = "the requested epoch's DKG ceremony has not concluded, so it has no signatures yet"),
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
        Some(raw) => Date::parse(&raw, RFC_3339_DATE_FORMAT)
            .map_err(|_| EcashError::MalformedExpirationDate { raw })?,
    };

    let epoch_id = state.requested_epoch(epoch_id).await?;

    // these signatures are an input to spending a ticketbook from that epoch, and they cannot
    // change once its ceremony is done - so a later ceremony must not withhold them
    state.ensure_ceremony_concluded(epoch_id).await?;

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
        )),
        (status = 400, body = String, description = "the requested epoch's DKG ceremony has not concluded, so it has no signatures yet"),
    ),
)]
async fn coin_indices_signatures(
    Query(EpochIdParam { epoch_id, output }): Query<EpochIdParam>,
    State(state): State<Arc<EcashState>>,
) -> AxumResult<FormattedResponse<AggregatedCoinIndicesSignatureResponse>> {
    trace!("aggregated_coin_indices_signatures request");

    let output = output.unwrap_or_default();

    let epoch_id = state.requested_epoch(epoch_id).await?;

    // as above: an input to spending, fixed once that epoch's ceremony concluded
    state.ensure_ceremony_concluded(epoch_id).await?;

    let coin_indices_signatures = state.master_coin_index_signatures(Some(epoch_id)).await?;

    Ok(output.to_response(AggregatedCoinIndicesSignatureResponse {
        epoch_id: coin_indices_signatures.epoch_id,
        signatures: coin_indices_signatures.signatures.clone(),
    }))
}
