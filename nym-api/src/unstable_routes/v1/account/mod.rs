// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    node_status_api::models::{AxumErrorResponse, AxumResult},
    support::http::state::AppState,
};
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use models::NyxAccountDetails;
use nym_validator_client::nyxd::AccountId;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{error, instrument};
use utoipa::ToSchema;

pub(crate) mod cache;
pub(crate) mod data_collector;
pub(crate) mod models;

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/:address", get(address))
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct AddressQueryParam {
    #[serde(default)]
    pub address: String,
}

#[utoipa::path(
    tag = "Unstable",
    get,
    path = "/{address}",
    context_path = "/v1/unstable/account",
    responses(
        (status = 200, body = NyxAccountDetails)
    ),
    params(AddressQueryParam)
)]
#[instrument(level = "info", skip_all, fields(address=address))]
async fn address(
    Path(AddressQueryParam { address }): Path<AddressQueryParam>,
    State(state): State<AppState>,
) -> AxumResult<Json<NyxAccountDetails>> {
    let account_id = AccountId::from_str(&address).map_err(|err| {
        error!("{err}");
        AxumErrorResponse::not_found(&address)
    })?;

    state.get_address_info(account_id).await.map(Json)
}
