// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network::models::{ContractInformation, NetworkDetails};
use crate::node_status_api::models::AxumResult;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::{extract, Router};
use nym_api_requests::models::ChainStatusResponse;
use nym_contracts_common::ContractBuildInformation;
use nym_http_api_common::{FormattedResponse, OutputParams};
use std::collections::HashMap;
use tower_http::compression::CompressionLayer;
use utoipa::ToSchema;

pub(crate) fn nym_network_routes() -> Router<AppState> {
    Router::new()
        .route("/details", axum::routing::get(network_details))
        .route("/chain-status", axum::routing::get(chain_status))
        .route("/nym-contracts", axum::routing::get(nym_contracts))
        .route(
            "/nym-contracts-detailed",
            axum::routing::get(nym_contracts_detailed),
        )
        .layer(CompressionLayer::new())
}

#[utoipa::path(
    tag = "network",
    get,
    path = "/v1/network/details",
    responses(
        (status = 200, content(
            (NetworkDetails = "application/json"),
            (NetworkDetails = "application/yaml"),
            (NetworkDetails = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn network_details(
    Query(output): Query<OutputParams>,
    extract::State(state): extract::State<AppState>,
) -> FormattedResponse<NetworkDetails> {
    let output = output.output.unwrap_or_default();

    output.to_response(state.network_details().to_owned())
}

#[utoipa::path(
    tag = "network",
    get,
    path = "/v1/network/chain-status",
    responses(
        (status = 200, content(
            (ChainStatusResponse = "application/json"),
            (ChainStatusResponse = "application/yaml"),
            (ChainStatusResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn chain_status(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<ChainStatusResponse>> {
    let output = output.output.unwrap_or_default();

    let chain_status = state
        .chain_status_cache
        .get_or_refresh(&state.nyxd_client)
        .await?;

    let connected_nyxd = state.network_details.connected_nyxd;

    Ok(output.to_response(ChainStatusResponse {
        connected_nyxd,
        status: chain_status,
    }))
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

#[allow(dead_code)] // not dead, used in OpenAPI docs
#[derive(ToSchema)]
pub struct ContractInformationContractVersion {
    pub(crate) address: Option<String>,
    pub(crate) details: Option<ContractVersionSchemaResponse>,
}

#[utoipa::path(
    tag = "network",
    get,
    path = "/v1/network/nym-contracts",
    responses(
        (status = 200, content(
            (HashMap<String, ContractInformationContractVersion> = "application/json"),
            (HashMap<String, ContractInformationContractVersion> = "application/yaml"),
            (HashMap<String, ContractInformationContractVersion> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn nym_contracts(
    Query(output): Query<OutputParams>,
    extract::State(state): extract::State<AppState>,
) -> FormattedResponse<HashMap<String, ContractInformation<cw2::ContractVersion>>> {
    let output = output.output.unwrap_or_default();

    let info = state.nym_contract_cache().contract_details().await;
    output.to_response(
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
            .collect::<HashMap<_, _>>(),
    )
}

#[allow(dead_code)] // not dead, used in OpenAPI docs
#[derive(ToSchema)]
pub struct ContractInformationBuildInformation {
    pub(crate) address: Option<String>,
    pub(crate) details: Option<ContractBuildInformation>,
}

#[utoipa::path(
    tag = "network",
    get,
    path = "/v1/network/nym-contracts-detailed",
    responses(
        (status = 200, content(
            (HashMap<String, ContractInformationBuildInformation> = "application/json"),
            (HashMap<String, ContractInformationBuildInformation> = "application/yaml"),
            (HashMap<String, ContractInformationBuildInformation> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn nym_contracts_detailed(
    Query(output): Query<OutputParams>,
    extract::State(state): extract::State<AppState>,
) -> FormattedResponse<HashMap<String, ContractInformation<ContractBuildInformation>>> {
    let output = output.output.unwrap_or_default();

    let info = state.nym_contract_cache().contract_details().await;
    output.to_response(
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
            .collect::<HashMap<_, _>>(),
    )
}
