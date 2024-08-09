// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network::models::{ContractInformation, NetworkDetails};
use crate::v2::AxumAppState;
use axum::{extract, Router};
use nym_contracts_common::ContractBuildInformation;
use std::collections::HashMap;

pub(crate) fn nym_network_routes() -> Router<AxumAppState> {
    Router::new()
        .route("/details", axum::routing::get(network_details))
        .route("/nym-contracts", axum::routing::get(nym_contracts))
        .route(
            "/nym-contracts-detailed",
            axum::routing::get(nym_contracts_detailed),
        )
}

async fn network_details(
    extract::State(state): extract::State<AxumAppState>,
) -> axum::Json<NetworkDetails> {
    state.network_details().to_owned().into()
}

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
