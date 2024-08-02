// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::api_routes::helpers::build_credentials_response;
use crate::ecash::error::EcashError;
use crate::ecash::state::EcashState;
use crate::ecash::storage::EcashStorageExt;
use nym_api_requests::ecash::models::{
    EpochCredentialsResponse, IssuedCredentialResponse, IssuedCredentialsResponse,
};
use nym_api_requests::ecash::CredentialsRequestBody;
use nym_coconut_dkg_common::types::EpochId;
use rocket::serde::json::Json;
use rocket::State as RocketState;
use rocket_okapi::openapi;

#[openapi(tag = "Ecash")]
#[get("/epoch-credentials/<epoch>")]
pub async fn epoch_credentials(
    epoch: EpochId,
    state: &RocketState<EcashState>,
) -> crate::ecash::error::Result<Json<EpochCredentialsResponse>> {
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

#[openapi(tag = "Ecash")]
#[get("/issued-credential/<id>")]
pub async fn issued_credential(
    id: i64,
    state: &RocketState<EcashState>,
) -> crate::ecash::error::Result<Json<IssuedCredentialResponse>> {
    let issued = state.aux.storage.get_issued_credential(id).await?;

    let credential = if let Some(issued) = issued {
        Some(issued.try_into()?)
    } else {
        None
    };

    Ok(Json(IssuedCredentialResponse { credential }))
}

#[openapi(tag = "Ecash")]
#[post("/issued-credentials", data = "<params>")]
pub async fn issued_credentials(
    params: Json<CredentialsRequestBody>,
    state: &RocketState<EcashState>,
) -> crate::ecash::error::Result<Json<IssuedCredentialsResponse>> {
    let params = params.into_inner();

    if params.pagination.is_some() && !params.credential_ids.is_empty() {
        return Err(EcashError::InvalidQueryArguments);
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

    build_credentials_response(credentials).map(Json)
}
