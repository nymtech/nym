// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::state::EcashState;
use crate::node_status_api::models::AxumResult;
use crate::support::http::state::AppState;
use axum::extract::{Path, State};
use axum::{Json, Router};
use nym_api_requests::ecash::models::{
    IssuedTicketbooksChallengeCommitmentRequest, IssuedTicketbooksChallengeCommitmentResponse,
    IssuedTicketbooksChallengeRequest, IssuedTicketbooksChallengeResponse,
    IssuedTicketbooksDataRequest, IssuedTicketbooksDataResponse, IssuedTicketbooksForResponse,
    SignableMessageBody,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::Date;
use tracing::trace;
use utoipa::{IntoParams, ToSchema};

pub(crate) fn issued_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/issued-ticketbooks-for/:expiration_date",
            axum::routing::get(issued_ticketbooks_for),
        )
        .route(
            "/issued-ticketbooks-challenge-commitment",
            axum::routing::post(issued_ticketbooks_challenge_commitment),
        )
        .route(
            "//issued-ticketbooks-data",
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

async fn number_of_issued_ticketbooks_for() {
    todo!()
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
