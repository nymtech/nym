// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::router::api::v1::ticketbook::FormattedTicketbookWalletSharesResponse;
use crate::http::state::ApiState;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::Router;
use nym_credential_proxy_lib::helpers::random_uuid;
use nym_credential_proxy_lib::http_helpers::RequestError;
use nym_credential_proxy_requests::api::v1::ticketbook::models::{
    SharesQueryParams, TicketbookWalletSharesResponse,
};
use nym_credential_proxy_requests::routes::api::v1::ticketbook::shares;
use nym_http_api_common::OutputParams;

/// Query by id for blinded shares of a bandwidth voucher
#[utoipa::path(
    get,
    path = "/{share_id}",
    context_path = "/api/v1/ticketbook/shares",
    tag = "Ticketbook Wallet Shares",
    responses(
        (status = 200, content(
            (TicketbookWalletSharesResponse = "application/json"),
            (TicketbookWalletSharesResponse = "application/yaml"),
        )),
        (status = 404, description = "share_id not found"),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 500, body = String, description = "failed to query for bandwidth blinded shares"),
    ),
    params(OutputParams),
    security(
        ("auth_token" = [])
    )
)]
pub(crate) async fn query_for_shares_by_id(
    State(state): State<ApiState>,
    Query(params): Query<SharesQueryParams>,
    Path(share_id): Path<i64>,
) -> Result<FormattedTicketbookWalletSharesResponse, RequestError> {
    let uuid = random_uuid();
    let output = params.output.unwrap_or_default();

    let response = state
        .inner_state()
        .query_for_shares_by_id(uuid, params.global, share_id)
        .await
        .map_err(|err| RequestError::new_server_error(err, uuid))?;

    Ok(output.to_response(response))
}

/// Query by id for blinded  wallet shares of a ticketbook
#[utoipa::path(
    get,
    path = "/device/{device_id}/credential/{credential_id}",
    context_path = "/api/v1/ticketbook/shares",
    tag = "Ticketbook Wallet Shares",
    responses(
        (status = 200, content(
            (TicketbookWalletSharesResponse = "application/json"),
            (TicketbookWalletSharesResponse = "application/yaml"),
        )),
        (status = 404, description = "share_id not found"),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 500, body = String, description = "failed to query for bandwidth blinded shares"),
    ),
    params(SharesQueryParams),
    security(
        ("auth_token" = [])
    )
)]
pub(crate) async fn query_for_shares_by_device_id_and_credential_id(
    State(state): State<ApiState>,
    Query(params): Query<SharesQueryParams>,
    Path((device_id, credential_id)): Path<(String, String)>,
) -> Result<FormattedTicketbookWalletSharesResponse, RequestError> {
    let uuid = random_uuid();
    let output = params.output.unwrap_or_default();

    let response = state
        .inner_state()
        .query_for_shares_by_device_id_and_credential_id(
            uuid,
            params.global,
            device_id,
            credential_id,
        )
        .await
        .map_err(|err| RequestError::new_server_error(err, uuid))?;

    Ok(output.to_response(response))
}

pub(crate) fn routes() -> Router<ApiState> {
    Router::new()
        .route(shares::SHARE_BY_ID, get(query_for_shares_by_id))
        .route(
            shares::SHARE_BY_DEVICE_AND_CREDENTIAL_ID,
            get(query_for_shares_by_device_id_and_credential_id),
        )
}
