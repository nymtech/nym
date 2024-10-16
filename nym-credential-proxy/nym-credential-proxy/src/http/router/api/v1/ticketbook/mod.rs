// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::credentials::ticketbook::{
    try_obtain_blinded_ticketbook_async, try_obtain_wallet_shares,
};
use crate::http::helpers::random_uuid;
use crate::http::state::ApiState;
use crate::http::types::RequestError;
use crate::nym_api_helpers::ensure_sane_expiration_date;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use nym_compact_ecash::Base58;
use nym_credential_proxy_requests::api::v1::ticketbook::models::{
    CurrentEpochResponse, DepositResponse, MasterVerificationKeyResponse, PartialVerificationKey,
    PartialVerificationKeysResponse, TicketbookAsyncRequest, TicketbookObtainQueryParams,
    TicketbookRequest, TicketbookWalletSharesAsyncResponse, TicketbookWalletSharesResponse,
};
use nym_credential_proxy_requests::routes::api::v1::ticketbook;
use nym_http_api_common::{FormattedResponse, OutputParams};
use time::OffsetDateTime;
use tracing::{error, info, span, warn, Level};

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
            ("application/json" = TicketbookWalletSharesResponse),
            ("application/yaml" = TicketbookWalletSharesResponse),
        )),
        (status = 400, description = "the provided request hasn't been created against correct attributes"),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 422, description = "provided request was malformed"),
        (status = 500, body = ErrorResponse, description = "failed to obtain a ticketbook"),
        (status = 503, body = ErrorResponse, description = "ticketbooks can't be issued at this moment: the epoch transition is probably taking place"),
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
    let requested_on = OffsetDateTime::now_utc();

    let span = span!(Level::INFO, "obtain ticketboook", uuid = %uuid);
    let _entered = span.enter();
    info!("");

    let output = params.output.unwrap_or_default();

    state.ensure_not_in_epoch_transition(Some(uuid)).await?;
    let epoch_id = state
        .current_epoch_id()
        .await
        .map_err(|err| RequestError::new_server_error(err, uuid))?;

    if let Err(err) = ensure_sane_expiration_date(payload.expiration_date) {
        warn!("failure due to invalid expiration date");
        return Err(RequestError::new_with_uuid(
            err.to_string(),
            uuid,
            StatusCode::BAD_REQUEST,
        ));
    }

    // if additional data was requested, grab them first in case there are any cache/network issues
    let (
        master_verification_key,
        aggregated_expiration_date_signatures,
        aggregated_coin_index_signatures,
    ) = state
        .response_global_data(
            params.include_master_verification_key,
            params.include_expiration_date_signatures,
            params.include_coin_index_signatures,
            epoch_id,
            payload.expiration_date,
            uuid,
        )
        .await?;

    let shares = try_obtain_wallet_shares(&state, uuid, requested_on, payload)
        .await
        .inspect_err(|err| warn!("request failure: {err}"))
        .map_err(|err| RequestError::new(err.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

    info!("request was successful!");
    Ok(output.to_response(TicketbookWalletSharesResponse {
        epoch_id,
        shares,
        master_verification_key,
        aggregated_coin_index_signatures,
        aggregated_expiration_date_signatures,
    }))
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
            ("application/json" = TicketbookWalletSharesAsyncResponse),
            ("application/yaml" = TicketbookWalletSharesAsyncResponse),
        )),
        (status = 400, description = "the provided request hasn't been created against correct attributes"),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 409, description = "shares were already requested"),
        (status = 422, description = "provided request was malformed"),
        (status = 500, body = ErrorResponse, description = "failed to obtain a ticketbook"),
        (status = 503, body = ErrorResponse, description = "ticketbooks can't be issued at this moment: the epoch transition is probably taking place"),
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
    let requested_on = OffsetDateTime::now_utc();

    let span = span!(Level::INFO, "[async] obtain ticketboook", uuid = %uuid);
    let _entered = span.enter();
    info!("");

    let output = params.output.unwrap_or_default();

    // 1. perform basic validation
    state.ensure_not_in_epoch_transition(Some(uuid)).await?;

    if let Err(err) = ensure_sane_expiration_date(payload.inner.expiration_date) {
        warn!("failure due to invalid expiration date");
        return Err(RequestError::new_with_uuid(
            err.to_string(),
            uuid,
            StatusCode::BAD_REQUEST,
        ));
    }

    // 2. store the request to retrieve the id
    let pending = match state
        .storage()
        .insert_new_pending_async_shares_request(uuid, &payload.device_id, &payload.credential_id)
        .await
    {
        Err(err) => {
            error!("failed to insert new pending async shares: {err}");
            return Err(RequestError::new_with_uuid(
                err.to_string(),
                uuid,
                StatusCode::CONFLICT,
            ));
        }
        Ok(pending) => pending,
    };

    // 3. try to spawn a new task attempting to resolve the request
    if state
        .try_spawn(try_obtain_blinded_ticketbook_async(
            state.clone(),
            uuid,
            requested_on,
            payload,
            params,
        ))
        .is_none()
    {
        // we're going through the shutdown
        return Err(RequestError::new_with_uuid(
            "server shutdown in progress",
            uuid,
            StatusCode::INTERNAL_SERVER_ERROR,
        ));
    }

    // 4. in the meantime, return the id to the user
    Ok(output.to_response(TicketbookWalletSharesAsyncResponse {
        id: pending.id,
        uuid,
    }))
}

/// Obtain the current value of the bandwidth voucher deposit
#[utoipa::path(
    get,
    path = "/deposit-amount",
    context_path = "/api/v1/ticketbook",
    tag = "Ticketbook",
    responses(
        (status = 200, content(
            ("application/json" = DepositResponse),
            ("application/yaml" = DepositResponse),
        )),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 500, body = ErrorResponse, description = "failed to obtain current deposit information"),
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
    let current_deposit = state
        .deposit_amount()
        .await
        .map_err(|err| RequestError::new(err.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(output.to_response(DepositResponse {
        current_deposit_amount: current_deposit.amount,
        current_deposit_denom: current_deposit.denom,
    }))
}

/// Obtain partial verification keys of all signers for the current epoch.
#[utoipa::path(
    get,
    path = "/partial-verification-keys",
    context_path = "/api/v1/ticketbook",
    tag = "Ticketbook",
    responses(
        (status = 200, content(
            ("application/json" = PartialVerificationKeysResponse),
            ("application/yaml" = PartialVerificationKeysResponse),
        )),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 500, body = ErrorResponse, description = "failed to obtain current epoch information"),
        (status = 503, body = ErrorResponse, description = "credentials can't be issued at this moment: the epoch transition is probably taking place"),
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

    state.ensure_not_in_epoch_transition(None).await?;

    let epoch_id = state
        .current_epoch_id()
        .await
        .map_err(|err| RequestError::new(err.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

    let signers = state
        .ecash_clients(epoch_id)
        .await
        .map_err(|err| RequestError::new(err.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(output.to_response(PartialVerificationKeysResponse {
        epoch_id,
        keys: signers
            .iter()
            .map(|signer| PartialVerificationKey {
                node_index: signer.node_id,
                bs58_encoded_key: signer.verification_key.to_bs58(),
            })
            .collect(),
    }))
}

/// Obtain the master verification key for the current epoch.
#[utoipa::path(
    get,
    path = "/master-verification-key",
    context_path = "/api/v1/ticketbook",
    tag = "Ticketbook",
    responses(
        (status = 200, content(
            ("application/json" = MasterVerificationKeyResponse),
            ("application/yaml" = MasterVerificationKeyResponse),
        )),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 500, body = ErrorResponse, description = "failed to obtain current epoch information"),
        (status = 503, body = ErrorResponse, description = "credentials can't be issued at this moment: the epoch transition is probably taking place"),
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

    state.ensure_not_in_epoch_transition(None).await?;

    let epoch_id = state
        .current_epoch_id()
        .await
        .map_err(|err| RequestError::new(err.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

    let key = state
        .master_verification_key(Some(epoch_id))
        .await
        .map_err(|err| RequestError::new(err.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(output.to_response(MasterVerificationKeyResponse {
        epoch_id,
        bs58_encoded_key: key.to_bs58(),
    }))
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
            ("application/json" = CurrentEpochResponse),
            ("application/yaml" = CurrentEpochResponse),
        )),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 500, body = ErrorResponse, description = "failed to obtain current epoch information"),
        (status = 503, body = ErrorResponse, description = "credentials can't be issued at this moment: the epoch transition is probably taking place"),
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

    state.ensure_not_in_epoch_transition(None).await?;

    let epoch_id = state
        .current_epoch_id()
        .await
        .map_err(|err| RequestError::new(err.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(output.to_response(CurrentEpochResponse { epoch_id }))
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
