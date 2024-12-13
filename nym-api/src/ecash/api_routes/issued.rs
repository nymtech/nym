// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::state::EcashState;
use crate::node_status_api::models::AxumResult;
use crate::support::http::state::AppState;
use axum::extract::{Path, State};
use axum::{Json, Router};
use nym_api_requests::ecash::models::{
    IssuedTicketbooksChallengeRequest, IssuedTicketbooksChallengeResponse,
    IssuedTicketbooksForResponse,
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
            "/issued-ticketbooks-challenge",
            axum::routing::post(issued_ticketbooks_challenge),
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
        (status = 400, body = ErrorResponse, description = "this nym-api is not an ecash signer in the current epoch"),
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
    request_body = IssuedTicketbooksChallengeRequest,
    path = "/issued-ticketbooks-challenge",
    context_path = "/v1/ecash",
    responses(
        (status = 200, body = IssuedTicketbooksChallengeResponse),
        (status = 400, body = ErrorResponse, description = "this nym-api is not an ecash signer in the current epoch"),
    )
)]
async fn issued_ticketbooks_challenge(
    State(state): State<Arc<EcashState>>,
    Json(challenge): Json<IssuedTicketbooksChallengeRequest>,
) -> AxumResult<Json<IssuedTicketbooksChallengeResponse>> {
    trace!("replying to ticketbooks challenge: {:?}", challenge);
    state.ensure_signer().await?;

    Ok(Json(
        state
            .get_issued_ticketbooks(challenge)
            .await?
            .sign(state.local.identity_keypair.private_key()),
    ))
}
