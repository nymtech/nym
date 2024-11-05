// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::VpnApiError;
use crate::http::helpers::random_uuid;
use crate::http::router::api::v1::ticketbook::FormattedTicketbookWalletSharesResponse;
use crate::http::state::ApiState;
use crate::http::types::RequestError;
use crate::storage::models::MinimalWalletShare;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use nym_credential_proxy_requests::api::v1::ticketbook::models::{
    SharesQueryParams, TicketbookWalletSharesResponse,
};
use nym_credential_proxy_requests::routes::api::v1::ticketbook::shares;
use nym_http_api_common::OutputParams;
use nym_validator_client::nym_api::EpochId;
use tracing::{debug, span, warn, Level};
use uuid::Uuid;

async fn shares_to_response(
    state: ApiState,
    uuid: Uuid,
    shares: Vec<MinimalWalletShare>,
    params: SharesQueryParams,
) -> Result<FormattedTicketbookWalletSharesResponse, RequestError> {
    // in all calls we ensured the shares are non-empty
    #[allow(clippy::unwrap_used)]
    let first = shares.first().unwrap();
    let expiration_date = first.expiration_date;
    let epoch_id = first.epoch_id as EpochId;

    let threshold = state.response_ecash_threshold(uuid, epoch_id).await?;
    if shares.len() < threshold as usize {
        return Err(RequestError::new_server_error(
            VpnApiError::InsufficientNumberOfCredentials {
                available: shares.len(),
                threshold,
            },
            uuid,
        ));
    }

    // grab any requested additional data
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
            expiration_date,
            uuid,
        )
        .await?;

    // finally produce a response
    Ok(params
        .output
        .unwrap_or_default()
        .to_response(TicketbookWalletSharesResponse {
            epoch_id,
            shares: shares.into_iter().map(Into::into).collect(),
            master_verification_key,
            aggregated_coin_index_signatures,
            aggregated_expiration_date_signatures,
        }))
}

/// Query by id for blinded shares of a bandwidth voucher
#[utoipa::path(
    get,
    path = "/{share_id}",
    context_path = "/api/v1/ticketbook/shares",
    tag = "Ticketbook Wallet Shares",
    responses(
        (status = 200, content(
            ("application/json" = TicketbookWalletSharesResponse),
            ("application/yaml" = TicketbookWalletSharesResponse),
        )),
        (status = 404, description = "share_id not found"),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 500, body = ErrorResponse, description = "failed to query for bandwidth blinded shares"),
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

    let span = span!(Level::INFO, "query shares by id", uuid = %uuid, share_id = %share_id);
    let _entered = span.enter();
    debug!("");

    // TODO: edge case: this will **NOT** work if shares got created in epoch X,
    // but this query happened in epoch X+1
    let shares = match state
        .storage()
        .load_wallet_shares_by_shares_id(share_id)
        .await
    {
        Ok(shares) => {
            if shares.is_empty() {
                debug!("not found");
                return Err(RequestError::new_with_uuid(
                    format!("not found - share_id = {share_id}"),
                    uuid,
                    StatusCode::NOT_FOUND,
                ));
            }
            shares
        }
        Err(err) => {
            warn!("db failure: {err}");
            return Err(RequestError::new_with_uuid(
                format!("oh no, something went wrong {err}"),
                uuid,
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    shares_to_response(state, uuid, shares, params).await
}

/// Query by id for blinded  wallet shares of a ticketbook
#[utoipa::path(
    get,
    path = "/device/{device_id}/credential/{credential_id}",
    context_path = "/api/v1/ticketbook/shares",
    tag = "Ticketbook Wallet Shares",
    responses(
        (status = 200, content(
            ("application/json" = TicketbookWalletSharesResponse),
            ("application/yaml" = TicketbookWalletSharesResponse),
        )),
        (status = 404, description = "share_id not found"),
        (status = 401, description = "authentication token is missing or is invalid"),
        (status = 500, body = ErrorResponse, description = "failed to query for bandwidth blinded shares"),
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

    let span = span!(Level::INFO, "query shares by device and credential ids", uuid = %uuid, device_id = %device_id, credential_id = %credential_id);
    let _entered = span.enter();
    debug!("");

    // TODO: edge case: this will **NOT** work if shares got created in epoch X,
    // but this query happened in epoch X+1
    let shares = match state
        .storage()
        .load_wallet_shares_by_device_and_credential_id(&device_id, &credential_id)
        .await
    {
        Ok(shares) => {
            if shares.is_empty() {
                debug!("not found");
                return Err(RequestError::new_with_uuid(
                    format!("not found - device_id = {device_id}, credential_id = {credential_id}"),
                    uuid,
                    StatusCode::NOT_FOUND,
                ));
            }
            shares
        }
        Err(err) => {
            warn!("db failure: {err}");
            return Err(RequestError::new_with_uuid(
                format!("oh no, something went wrong {err}"),
                uuid,
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    shares_to_response(state, uuid, shares, params).await
}

pub(crate) fn routes() -> Router<ApiState> {
    Router::new()
        .route(shares::SHARE_BY_ID, get(query_for_shares_by_id))
        .route(
            shares::SHARE_BY_DEVICE_AND_CREDENTIAL_ID,
            get(query_for_shares_by_device_id_and_credential_id),
        )
}
