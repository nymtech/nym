// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::state::EcashState;
use crate::node_status_api::models::AxumResult;
use crate::support::http::helpers::PaginationRequest;
use crate::support::http::state::AppState;
use axum::extract::{Path, Query, State};
use axum::{Json, Router};
use nym_api_requests::ecash::models::{
    IssuedTicketbooksChallengeCommitmentRequest, IssuedTicketbooksChallengeCommitmentResponse,
    IssuedTicketbooksCountResponse, IssuedTicketbooksDataRequest, IssuedTicketbooksDataResponse,
    IssuedTicketbooksForCountResponse, IssuedTicketbooksForResponse,
    IssuedTicketbooksOnCountResponse, SignableMessageBody,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::sync::Arc;
use time::Date;
use utoipa::{IntoParams, ToSchema};

const MAX_ISSUANCE_COUNT_PAGE_SIZE: u32 = 100;
const DEFAULT_ISSUANCE_COUNT_PAGE_SIZE: u32 = 50;

pub(crate) fn issued_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/issued-ticketbooks-count",
            axum::routing::post(issued_ticketbooks_count),
        )
        .route(
            "/issued-ticketbooks-for-count/:expiration_date",
            axum::routing::get(issued_ticketbooks_for_count),
        )
        .route(
            "/issued-ticketbooks-on-count/:issuance_date",
            axum::routing::get(issued_ticketbooks_on_count),
        )
        .route(
            "/issued-ticketbooks-for/:expiration_date",
            axum::routing::get(issued_ticketbooks_for),
        )
        .route(
            "/issued-ticketbooks-challenge-commitment",
            axum::routing::post(issued_ticketbooks_challenge_commitment),
        )
        .route(
            "/issued-ticketbooks-data",
            axum::routing::post(issued_ticketbooks_data),
        )
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, IntoParams, ToSchema, JsonSchema)]
#[into_params(parameter_in = Path)]
pub(crate) struct ExpirationDatePathParam {
    #[schema(value_type = String, example = "1970-01-01")]
    #[schemars(with = "String")]
    #[serde(with = "nym_serde_helpers::date")]
    pub(crate) expiration_date: Date,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, IntoParams, ToSchema, JsonSchema)]
#[into_params(parameter_in = Path)]
pub(crate) struct IssuanceDatePathParam {
    #[schema(value_type = String, example = "1970-01-01")]
    #[schemars(with = "String")]
    #[serde(with = "nym_serde_helpers::date")]
    pub(crate) issuance_date: Date,
}

/// Returns number of issued ticketbooks for given dates.
/// Note: this endpoint will not be accurate for old dates
#[utoipa::path(
    tag = "Ecash",
    get,
    params(
        PaginationRequest
    ),
    path = "/issued-ticketbooks-count",
    context_path = "/v1/ecash",
    responses(
        (status = 200, body = IssuedTicketbooksCountResponse),
        (status = 400, body = String, description = "this nym-api is not an ecash signer in the current epoch"),
    )
)]
async fn issued_ticketbooks_count(
    Query(pagination): Query<PaginationRequest>,
    State(state): State<Arc<EcashState>>,
) -> AxumResult<Json<IssuedTicketbooksCountResponse>> {
    state.ensure_signer().await?;

    let page = pagination.page.unwrap_or_default();
    let per_page = min(
        pagination
            .per_page
            .unwrap_or(DEFAULT_ISSUANCE_COUNT_PAGE_SIZE),
        MAX_ISSUANCE_COUNT_PAGE_SIZE,
    );

    Ok(Json(
        state.get_issued_ticketbooks_count(page, per_page).await?,
    ))
}

/// Returns number of issued ticketbooks for particular expiration date.
/// Note: this endpoint will not be accurate for old dates
#[utoipa::path(
    tag = "Ecash",
    get,
    params(
        ExpirationDatePathParam
    ),
    path = "/issued-ticketbooks-for-count/{expiration_date}",
    context_path = "/v1/ecash",
    responses(
        (status = 200, body = IssuedTicketbooksForCountResponse),
        (status = 400, body = String, description = "this nym-api is not an ecash signer in the current epoch"),
    )
)]
async fn issued_ticketbooks_for_count(
    Path(ExpirationDatePathParam { expiration_date }): Path<ExpirationDatePathParam>,
    State(state): State<Arc<EcashState>>,
) -> AxumResult<Json<IssuedTicketbooksForCountResponse>> {
    state.ensure_signer().await?;

    Ok(Json(
        state
            .get_issued_ticketbooks_for_count(expiration_date)
            .await?,
    ))
}

/// Returns number of issued ticketbooks on particular date.
/// Note: this endpoint will not be accurate for old dates
#[utoipa::path(
    tag = "Ecash",
    get,
    params(
        IssuanceDatePathParam
    ),
    path = "/issued-ticketbooks-on-count/{issuance_date}",
    context_path = "/v1/ecash",
    responses(
        (status = 200, body = IssuedTicketbooksOnCountResponse),
        (status = 400, body = String, description = "this nym-api is not an ecash signer in the current epoch"),
    )
)]
async fn issued_ticketbooks_on_count(
    Path(IssuanceDatePathParam { issuance_date }): Path<IssuanceDatePathParam>,
    State(state): State<Arc<EcashState>>,
) -> AxumResult<Json<IssuedTicketbooksOnCountResponse>> {
    state.ensure_signer().await?;

    Ok(Json(
        state.get_issued_ticketbooks_on_count(issuance_date).await?,
    ))
}

#[utoipa::path(
    tag = "Ecash",
    get,
    params(
        ExpirationDatePathParam
    ),
    path = "/issued-ticketbooks-for/{expiration_date}",
    context_path = "/v1/ecash",
    responses(
        (status = 200, body = IssuedTicketbooksForResponse),
        (status = 400, body = String, description = "this nym-api is not an ecash signer in the current epoch"),
    )
)]
async fn issued_ticketbooks_for(
    State(state): State<Arc<EcashState>>,
    Path(ExpirationDatePathParam { expiration_date }): Path<ExpirationDatePathParam>,
) -> AxumResult<Json<IssuedTicketbooksForResponse>> {
    state.ensure_signer().await?;

    Ok(Json(
        state
            .get_issued_ticketbooks_deposits_on(expiration_date)
            .await?
            .sign(state.local.identity_keypair.private_key()),
    ))
}

#[utoipa::path(
    tag = "Ecash",
    post,
    request_body = IssuedTicketbooksChallengeCommitmentRequest,
    path = "/issued-ticketbooks-challenge-commitment",
    context_path = "/v1/ecash",
    responses(
        (status = 200, body = IssuedTicketbooksChallengeCommitmentResponse),
        (status = 400, body = String, description = "this nym-api is not an ecash signer in the current epoch"),
    )
)]
async fn issued_ticketbooks_challenge_commitment(
    State(state): State<Arc<EcashState>>,
    Json(request): Json<IssuedTicketbooksChallengeCommitmentRequest>,
) -> AxumResult<Json<IssuedTicketbooksChallengeCommitmentResponse>> {
    state.ensure_signer().await?;

    Ok(Json(
        state
            .get_issued_ticketbooks_challenge_commitment(request)
            .await?
            .sign(state.local.identity_keypair.private_key()),
    ))
}

#[utoipa::path(
    tag = "Ecash",
    post,
    request_body = IssuedTicketbooksDataRequest,
    path = "/issued-ticketbooks-data",
    context_path = "/v1/ecash",
    responses(
        (status = 200, body = IssuedTicketbooksDataResponse),
        (status = 400, body = String, description = "this nym-api is not an ecash signer in the current epoch"),
    )
)]
async fn issued_ticketbooks_data(
    State(state): State<Arc<EcashState>>,
    Json(request): Json<IssuedTicketbooksDataRequest>,
) -> AxumResult<Json<IssuedTicketbooksDataResponse>> {
    state.ensure_signer().await?;

    Ok(Json(
        state
            .get_issued_ticketbooks_data(request)
            .await?
            .sign(state.local.identity_keypair.private_key()),
    ))
}
