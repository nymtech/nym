// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network::models::{ContractInformation, NetworkDetails};
use crate::v2::AxumAppState;
use axum::{extract, Router};
use nym_contracts_common::ContractBuildInformation;
use std::collections::HashMap;
use utoipa::ToSchema;

pub(crate) fn nym_network_routes() -> Router<AxumAppState> {
    Router::new()
        .route("/details", axum::routing::get(network_details))
        .route("/nym-contracts", axum::routing::get(nym_contracts))
        .route(
            "/nym-contracts-detailed",
            axum::routing::get(nym_contracts_detailed),
        )
}

#[utoipa::path(
    tag = "network",
    get,
    path = "/v1/network/details",
    responses(
        (status = 200, body = NetworkDetails)
    )
)]
async fn network_details(
    extract::State(state): extract::State<AxumAppState>,
) -> axum::Json<NetworkDetails> {
    state.network_details().to_owned().into()
}

// it's used for schema generation so dead_code is fine
#[allow(dead_code)]
#[derive(ToSchema)]
#[schema(title = "ContractVersion")]
pub(crate) struct ContractVersionSchemaResponse {
    /// contract is the crate name of the implementing contract, eg. `crate:cw20-base`
    /// we will use other prefixes for other languages, and their standard global namespacing
    pub contract: String,
    /// version is any string that this implementation knows. It may be simple counter "1", "2".
    /// or semantic version on release tags "v0.7.0", or some custom feature flag list.
    /// the only code that needs to understand the version parsing is code that knows how to
    /// migrate from the given contract (and is tied to it's implementation somehow)
    pub version: String,
}

#[utoipa::path(
    tag = "network",
    get,
    path = "/v1/network/nym-contracts",
    responses(
        (status = 200, body = HashMap<String, ContractInformation<ContractVersionSchemaResponse>>)
    )
)]
async fn nym_contracts(
    extract::State(state): extract::State<AxumAppState>,
) -> axum::Json<HashMap<String, ContractInformation<cw2::ContractVersion>>> {
    let info = state.nym_contract_cache().contract_details().await;
    info.iter()
        .map(|(contract, info)| {
            (
                contract.to_owned(),
                ContractInformation {
                    address: info.address.as_ref().map(|a| a.to_string()),
                    details: info.base.clone(),
                },
            )
        })
        .collect::<HashMap<_, _>>()
        .into()
}

#[utoipa::path(
    tag = "network",
    get,
    path = "/v1/network/nym-contracts-detailed",
    responses(
        (status = 200, body = HashMap<String, ContractInformation<ContractBuildInformation>>)
    )
)]
async fn nym_contracts_detailed(
    extract::State(state): extract::State<AxumAppState>,
) -> axum::Json<HashMap<String, ContractInformation<ContractBuildInformation>>> {
    let info = state.nym_contract_cache().contract_details().await;
    info.iter()
        .map(|(contract, info)| {
            (
                contract.to_owned(),
                ContractInformation {
                    address: info.address.as_ref().map(|a| a.to_string()),
                    details: info.detailed.clone(),
                },
            )
        })
        .collect::<HashMap<_, _>>()
        .into()
}
