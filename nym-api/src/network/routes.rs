// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network::models::{ContractInformation, NetworkDetails};
use crate::nym_contract_cache::cache::NymContractCache;
use nym_contracts_common::ContractBuildInformation;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use std::collections::HashMap;
use std::ops::Deref;

#[openapi(tag = "network")]
#[get("/details")]
pub(crate) fn network_details(details: &State<NetworkDetails>) -> Json<NetworkDetails> {
    Json(details.deref().clone())
}

// I agree, it feels weird to be pulling contract cache here, but I feel like it makes
// more sense to return this information here rather than in the generic cache route
#[openapi(tag = "network")]
#[get("/nym-contracts")]
pub(crate) async fn nym_contracts(
    cache: &State<NymContractCache>,
) -> Json<HashMap<String, ContractInformation<cw2::ContractVersion>>> {
    let info = cache.contract_details().await;
    Json(
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
            .collect(),
    )
}

#[openapi(tag = "network")]
#[get("/nym-contracts-detailed")]
pub(crate) async fn nym_contracts_detailed(
    cache: &State<NymContractCache>,
) -> Json<HashMap<String, ContractInformation<ContractBuildInformation>>> {
    let info = cache.contract_details().await;
    Json(
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
            .collect(),
    )
}
