// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::state::ApiState;
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use nym_credential_proxy_lib::helpers::random_uuid;
use nym_credential_proxy_lib::http_helpers::RequestError;
use nym_credential_proxy_requests::api::v1::ticketbook::models::{
    AggregatedCoinIndicesSignaturesResponse, AggregatedExpirationDateSignaturesResponse,
    CurrentEpochResponse, DepositResponse, EpochIdParams, ExpirationDateParams,
    MasterVerificationKeyResponse, ObtainTicketBookSharesAsyncResponse,
    PartialVerificationKeysResponse, TicketbookAsyncRequest, TicketbookObtainQueryParams,
    TicketbookRequest, TicketbookWalletSharesResponse,
};
use nym_credential_proxy_requests::routes::api::v1::ticketbook;
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_validator_client::nym_api::RFC_3339_DATE_FORMAT;
use reqwest::StatusCode;
use time::Date;

pub(crate) mod shares;

pub type FormattedDepositResponse = FormattedResponse<DepositResponse>;
pub type FormattedCurrentEpochResponse = FormattedResponse<CurrentEpochResponse>;
pub type FormattedMasterVerificationKeyResponse = FormattedResponse<MasterVerificationKeyResponse>;
pub type FormattedExpirationDateSignaturesResponse =
    FormattedResponse<AggregatedExpirationDateSignaturesResponse>;
pub type FormattedCoinIndexSignaturesResponse =
    FormattedResponse<AggregatedCoinIndicesSignaturesResponse>;
pub type FormattedPartialVerificationKeysResponse =
    FormattedResponse<PartialVerificationKeysResponse>;
pub type FormattedTicketbookWalletSharesResponse =
    FormattedResponse<TicketbookWalletSharesResponse>;
pub type FormattedTicketbookWalletSharesAsyncResponse =
    FormattedResponse<ObtainTicketBookSharesAsyncResponse>;

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
        .ticketbooks()
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
            (ObtainTicketBookSharesAsyncResponse = "application/json"),
            (ObtainTicketBookSharesAsyncResponse = "application/yaml"),
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

    // 0. check if we're in 'upgrade-mode' - if so, just return the attestation and associated jwt
    if let Some(upgrade_mode_response) = state.upgrade_mode_response().await {
        return Ok(output.to_response(upgrade_mode_response.into()));
    }

    let response = state
        .ticketbooks()
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
        .ticketbooks()
        .current_deposit()
        .await
        .map_err(RequestError::new_plain_error)?;

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
        .ticketbooks()
        .partial_verification_keys()
        .await
        .map_err(RequestError::new_plain_error)?;

    Ok(output.to_response(response))
}

/// Obtain the master verification key for the given or current epoch.
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
    params(EpochIdParams),
    security(
        ("auth_token" = [])
    )
)]
pub(crate) async fn master_verification_key(
    Query(EpochIdParams { epoch_id, output }): Query<EpochIdParams>,
    State(state): State<ApiState>,
) -> Result<FormattedMasterVerificationKeyResponse, RequestError> {
    let output = output.unwrap_or_default();

    let response = state
        .ticketbooks()
        .master_verification_key(epoch_id)
        .await
        .map_err(RequestError::new_plain_error)?;

    Ok(output.to_response(response))
}

/// Obtain the expiration date signatures for the given or current epoch and expiration date.
#[utoipa::path(
    get,
    path = "/aggregated-expiration-date-signatures",
    context_path = "/api/v1/ticketbook",
    tag = "Ticketbook",
    responses(
        (status = 200, content(
            (AggregatedExpirationDateSignaturesResponse = "application/json"),
            (AggregatedExpirationDateSignaturesResponse = "application/yaml"),
        )),
        (status = 400, body = String, description = "expiration_date is not a valid RFC3339 date"),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 500, body = String, description = "failed to obtain current epoch information"),
        (status = 503, body = String, description = "credentials can't be issued at this moment: the epoch transition is probably taking place"),
    ),
    params(ExpirationDateParams),
    security(
        ("auth_token" = [])
    )
)]
pub(crate) async fn expiration_date_signatures(
    Query(ExpirationDateParams {
        expiration_date,
        epoch_id,
        output,
    }): Query<ExpirationDateParams>,
    State(state): State<ApiState>,
) -> Result<FormattedExpirationDateSignaturesResponse, RequestError> {
    let output = output.unwrap_or_default();

    let expiration_date = expiration_date
        .map(|raw| {
            Date::parse(&raw, RFC_3339_DATE_FORMAT)
                .map_err(|err| RequestError::from_err(err, StatusCode::BAD_REQUEST))
        })
        .transpose()?;

    let response = state
        .ticketbooks()
        .master_expiration_date_signatures(epoch_id, expiration_date)
        .await
        .map_err(RequestError::new_plain_error)?;

    Ok(output.to_response(response))
}

/// Obtain the coin index signatures for the given or current epoch.
#[utoipa::path(
    get,
    path = "/aggregated-coin-indices-signatures",
    context_path = "/api/v1/ticketbook",
    tag = "Ticketbook",
    responses(
        (status = 200, content(
            (AggregatedCoinIndicesSignaturesResponse = "application/json"),
            (AggregatedCoinIndicesSignaturesResponse = "application/yaml"),
        )),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 500, body = String, description = "failed to obtain current epoch information"),
        (status = 503, body = String, description = "credentials can't be issued at this moment: the epoch transition is probably taking place"),
    ),
    params(EpochIdParams),
    security(
        ("auth_token" = [])
    )
)]
pub(crate) async fn coin_index_signatures(
    Query(EpochIdParams { epoch_id, output }): Query<EpochIdParams>,
    State(state): State<ApiState>,
) -> Result<FormattedCoinIndexSignaturesResponse, RequestError> {
    let output = output.unwrap_or_default();

    let response = state
        .ticketbooks()
        .master_coin_index_signatures(epoch_id)
        .await
        .map_err(RequestError::new_plain_error)?;

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
        .ticketbooks()
        .current_epoch()
        .await
        .map_err(RequestError::new_plain_error)?;

    Ok(output.to_response(response))
}

pub(crate) fn routes() -> Router<ApiState> {
    Router::new()
        .route(ticketbook::DEPOSIT_AMOUNT, get(current_deposit))
        .route(ticketbook::MASTER_KEY, get(master_verification_key))
        .route(
            ticketbook::AGGREGATED_EXPIRATION_DATE_SIGNATURES,
            get(expiration_date_signatures),
        )
        .route(
            ticketbook::AGGREGATED_COIN_INDICES_SIGNATURES,
            get(coin_index_signatures),
        )
        .route(ticketbook::PARTIAL_KEYS, get(partial_verification_keys))
        .route(ticketbook::CURRENT_EPOCH, get(current_epoch))
        .route(ticketbook::OBTAIN, post(obtain_ticketbook_shares))
        .route(
            ticketbook::OBTAIN_ASYNC,
            post(obtain_ticketbook_shares_async),
        )
        .nest(ticketbook::SHARES, shares::routes())
}
