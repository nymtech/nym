// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::state::ApiState;
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use nym_credential_proxy_lib::helpers::random_uuid;
use nym_credential_proxy_lib::http_helpers::RequestError;
use nym_credential_proxy_requests::api::v1::ticketbook::models::{
    CurrentEpochResponse, DepositResponse, MasterVerificationKeyResponse,
    PartialVerificationKeysResponse, TicketbookAsyncRequest, TicketbookObtainQueryParams,
    TicketbookRequest, TicketbookWalletSharesAsyncResponse, TicketbookWalletSharesResponse,
};
use nym_credential_proxy_requests::routes::api::v1::ticketbook;
use nym_http_api_common::{FormattedResponse, OutputParams};

pub(crate) mod shares;

pub type FormattedDepositResponse = FormattedResponse<DepositResponse>;
pub type FormattedCurrentEpochResponse = FormattedResponse<CurrentEpochResponse>;
pub type FormattedMasterVerificationKeyResponse = FormattedResponse<MasterVerificationKeyResponse>;
pub type FormattedPartialVerificationKeysResponse =
    FormattedResponse<PartialVerificationKeysResponse>;
pub type FormattedTicketbookWalletSharesResponse =
    FormattedResponse<TicketbookWalletSharesResponse>;
pub type FormattedTicketbookWalletSharesAsyncResponse =
    FormattedResponse<TicketbookWalletSharesAsyncResponse>;

/// Attempt to obtain blinded shares of an ecash ticketbook wallet
#[utoipa::path(
    post,
    path = "/obtain",
    context_path = "/api/v1/ticketbook",
    tag = "Ticketbook",
    request_body(
        content = TicketbookRequest,
        description = "cryptographic material required for obtaining ticketbook wallet shares",
        content_type = "application/json"
    ),
    responses(
        (status = 200, content(
            (TicketbookWalletSharesResponse = "application/json"),
            (TicketbookWalletSharesResponse = "application/yaml"),
        )),
        (status = 400, description = "the provided request hasn't been created against correct attributes"),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 422, description = "provided request was malformed"),
        (status = 500, body = String, description = "failed to obtain a ticketbook"),
        (status = 503, body = String, description = "ticketbooks can't be issued at this moment: the epoch transition is probably taking place"),
    ),
    params(TicketbookObtainQueryParams),
    security(
        ("auth_token" = [])
    )
)]
pub(crate) async fn obtain_ticketbook_shares(
    State(state): State<ApiState>,
    Query(params): Query<TicketbookObtainQueryParams>,
    Json(payload): Json<TicketbookRequest>,
) -> Result<FormattedTicketbookWalletSharesResponse, RequestError> {
    let uuid = random_uuid();
    let output = params.output.unwrap_or_default();

    let response = state
        .inner_state()
        .obtain_ticketbook_shares(uuid, payload, params.obtain_params.global)
        .await
        .map_err(|err| RequestError::new_server_error(err, uuid))?;

    Ok(output.to_response(response))
}

/// Attempt to obtain blinded shares of an ecash ticketbook wallet asynchronously
#[utoipa::path(
    post,
    path = "/obtain-async",
    context_path = "/api/v1/ticketbook",
    tag = "Ticketbook",
    request_body(
        content = TicketbookAsyncRequest,
        description = "cryptographic material required for obtaining ticketbook wallet shares",
        content_type = "application/json"
    ),
    responses(
        (status = 200, content(
            (TicketbookWalletSharesAsyncResponse = "application/json"),
            (TicketbookWalletSharesAsyncResponse = "application/yaml"),
        )),
        (status = 400, description = "the provided request hasn't been created against correct attributes"),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 409, description = "shares were already requested"),
        (status = 422, description = "provided request was malformed"),
        (status = 500, body = String, description = "failed to obtain a ticketbook"),
        (status = 503, body = String, description = "ticketbooks can't be issued at this moment: the epoch transition is probably taking place"),
    ),
    params(TicketbookObtainQueryParams),
    security(
        ("auth_token" = [])
    )
)]
pub(crate) async fn obtain_ticketbook_shares_async(
    State(state): State<ApiState>,
    Query(params): Query<TicketbookObtainQueryParams>,
    Json(payload): Json<TicketbookAsyncRequest>,
) -> Result<FormattedTicketbookWalletSharesAsyncResponse, RequestError> {
    let uuid = random_uuid();
    let output = params.output.unwrap_or_default();

    let response = state
        .inner_state()
        .obtain_ticketbook_shares_async(uuid, payload, params.obtain_params)
        .await
        .map_err(|err| RequestError::new_server_error(err, uuid))?;

    Ok(output.to_response(response))
}

/// Obtain the current value of the bandwidth voucher deposit
#[utoipa::path(
    get,
    path = "/deposit-amount",
    context_path = "/api/v1/ticketbook",
    tag = "Ticketbook",
    responses(
        (status = 200, content(
            (DepositResponse = "application/json"),
            (DepositResponse = "application/yaml"),
        )),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 500, body = String, description = "failed to obtain current deposit information"),
    ),
    params(OutputParams),
    security(
        ("auth_token" = [])
    )
)]
pub(crate) async fn current_deposit(
    Query(output): Query<OutputParams>,
    State(state): State<ApiState>,
) -> Result<FormattedDepositResponse, RequestError> {
    let output = output.output.unwrap_or_default();

    let response = state
        .inner_state()
        .current_deposit()
        .await
        .map_err(|err| RequestError::new_plain_error(err))?;

    Ok(output.to_response(response))
}

/// Obtain partial verification keys of all signers for the current epoch.
#[utoipa::path(
    get,
    path = "/partial-verification-keys",
    context_path = "/api/v1/ticketbook",
    tag = "Ticketbook",
    responses(
        (status = 200, content(
            (PartialVerificationKeysResponse = "application/json"),
            (PartialVerificationKeysResponse = "application/yaml"),
        )),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 500, body = String, description = "failed to obtain current epoch information"),
        (status = 503, body = String, description = "credentials can't be issued at this moment: the epoch transition is probably taking place"),
    ),
    params(OutputParams),
    security(
        ("auth_token" = [])
    )
)]
pub(crate) async fn partial_verification_keys(
    Query(output): Query<OutputParams>,
    State(state): State<ApiState>,
) -> Result<FormattedPartialVerificationKeysResponse, RequestError> {
    let output = output.output.unwrap_or_default();

    let response = state
        .inner_state()
        .partial_verification_keys()
        .await
        .map_err(|err| RequestError::new_plain_error(err))?;

    Ok(output.to_response(response))
}

/// Obtain the master verification key for the current epoch.
#[utoipa::path(
    get,
    path = "/master-verification-key",
    context_path = "/api/v1/ticketbook",
    tag = "Ticketbook",
    responses(
        (status = 200, content(
            (MasterVerificationKeyResponse = "application/json"),
            (MasterVerificationKeyResponse = "application/yaml"),
        )),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 500, body = String, description = "failed to obtain current epoch information"),
        (status = 503, body = String, description = "credentials can't be issued at this moment: the epoch transition is probably taking place"),
    ),
    params(OutputParams),
    security(
        ("auth_token" = [])
    )
)]
pub(crate) async fn master_verification_key(
    Query(output): Query<OutputParams>,
    State(state): State<ApiState>,
) -> Result<FormattedMasterVerificationKeyResponse, RequestError> {
    let output = output.output.unwrap_or_default();

    let response = state
        .inner_state()
        .master_verification_key()
        .await
        .map_err(|err| RequestError::new_plain_error(err))?;

    Ok(output.to_response(response))
}

/// Obtain the id of the current epoch.
/// This is exposed to allow clients to cache verification keys.
#[utoipa::path(
    get,
    path = "/current-epoch",
    context_path = "/api/v1/ticketbook",
    tag = "Ticketbook",
    responses(
        (status = 200, content(
            (CurrentEpochResponse = "application/json"),
            (CurrentEpochResponse = "application/yaml"),
        )),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 500, body = String, description = "failed to obtain current epoch information"),
        (status = 503, body = String, description = "credentials can't be issued at this moment: the epoch transition is probably taking place"),
    ),
    params(OutputParams),
    security(
        ("auth_token" = [])
    )
)]
pub(crate) async fn current_epoch(
    Query(output): Query<OutputParams>,
    State(state): State<ApiState>,
) -> Result<FormattedCurrentEpochResponse, RequestError> {
    let output = output.output.unwrap_or_default();

    let response = state
        .inner_state()
        .current_epoch()
        .await
        .map_err(|err| RequestError::new_plain_error(err))?;

    Ok(output.to_response(response))
}

pub(crate) fn routes() -> Router<ApiState> {
    Router::new()
        .route(ticketbook::DEPOSIT_AMOUNT, get(current_deposit))
        .route(ticketbook::MASTER_KEY, get(master_verification_key))
        .route(ticketbook::PARTIAL_KEYS, get(partial_verification_keys))
        .route(ticketbook::CURRENT_EPOCH, get(current_epoch))
        .route(ticketbook::OBTAIN, post(obtain_ticketbook_shares))
        .route(
            ticketbook::OBTAIN_ASYNC,
            post(obtain_ticketbook_shares_async),
        )
        .nest(ticketbook::SHARES, shares::routes())
}
