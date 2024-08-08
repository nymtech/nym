// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::api_routes::helpers::build_credentials_response;
use crate::ecash::error::EcashError;
use crate::ecash::state::EcashState;
use crate::ecash::storage::EcashStorageExt;
use crate::node_status_api::models::AxumResult;
use crate::v2::AxumAppState;
use axum::extract::Path;
use axum::{Json, Router};
use nym_api_requests::ecash::models::{
    EpochCredentialsResponse, IssuedCredentialResponse, IssuedCredentialsResponse,
};
use nym_api_requests::ecash::CredentialsRequestBody;
use nym_coconut_dkg_common::types::EpochId;
use std::sync::Arc;

pub(crate) fn issued_routes(ecash_state: Arc<EcashState>) -> Router<AxumAppState> {
    Router::new()
        .route(
            "/epoch-credentials/:epoch",
            axum::routing::get({
                let ecash_state = Arc::clone(&ecash_state);
                |epoch| epoch_credentials(epoch, ecash_state)
            }),
        )
        .route(
            "/issued-credential/:id",
            axum::routing::get({
                let ecash_state = Arc::clone(&ecash_state);
                |id| issued_credential(id, ecash_state)
            }),
        )
        .route(
            "/issued-credentials",
            axum::routing::post({
                let ecash_state = Arc::clone(&ecash_state);
                |body| issued_credentials(body, ecash_state)
            }),
        )
}

// #[openapi(tag = "Ecash")]
// #[get("/epoch-credentials/<epoch>")]
async fn epoch_credentials(
    Path(epoch): Path<EpochId>,
    state: Arc<EcashState>,
) -> AxumResult<Json<EpochCredentialsResponse>> {
    let issued = state.aux.storage.get_epoch_credentials(epoch).await?;

    let response = if let Some(issued) = issued {
        issued.into()
    } else {
        EpochCredentialsResponse {
            epoch_id: epoch,
            first_epoch_credential_id: None,
            total_issued: 0,
        }
    };

    Ok(Json(response))
}

// #[openapi(tag = "Ecash")]
// #[get("/issued-credential/<id>")]
async fn issued_credential(
    Path(id): Path<i64>,
    state: Arc<EcashState>,
) -> AxumResult<Json<IssuedCredentialResponse>> {
    let issued = state.aux.storage.get_issued_credential(id).await?;

    let credential = if let Some(issued) = issued {
        Some(issued.try_into()?)
    } else {
        None
    };

    Ok(Json(IssuedCredentialResponse { credential }))
}

// #[openapi(tag = "Ecash")]
// #[post("/issued-credentials", data = "<params>")]
async fn issued_credentials(
    Json(params): Json<CredentialsRequestBody>,
    state: Arc<EcashState>,
) -> AxumResult<Json<IssuedCredentialsResponse>> {
    if params.pagination.is_some() && !params.credential_ids.is_empty() {
        return Err(EcashError::InvalidQueryArguments.into());
    }

    let credentials = if let Some(pagination) = params.pagination {
        state
            .aux
            .storage
            .get_issued_credentials_paged(pagination)
            .await?
    } else {
        state
            .aux
            .storage
            .get_issued_credentials(params.credential_ids)
            .await?
    };

    build_credentials_response(credentials)
        .map(Json)
        .map_err(From::from)
}
