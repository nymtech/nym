// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use reqwest::Error as ReqwestError;
use rocket::response::status::NotFound;
use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;

use mixnet_contract_common::Delegation;

use crate::mix_node::models::{
    EconomicDynamicsStats, NodeDescription, NodeStats, PrettyDetailedMixNodeBond,
};
use crate::mix_nodes::delegations::get_single_mixnode_delegations;
use crate::state::ExplorerApiStateContext;

pub fn mix_node_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        settings: get_delegations,
        get_by_id,
        get_description,
        get_stats,
        get_economic_dynamics_stats,
    ]
}

#[openapi(tag = "mix_nodes")]
#[get("/<pubkey>")]
pub(crate) async fn get_by_id(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Result<Json<PrettyDetailedMixNodeBond>, NotFound<String>> {
    match state
        .inner
        .mixnodes
        .get_detailed_mixnode_by_id(pubkey)
        .await
    {
        Some(mixnode) => Ok(Json(mixnode)),
        None => Err(NotFound("Mixnode not found".to_string())),
    }
}

#[openapi(tag = "mix_node")]
#[get("/<pubkey>/delegations")]
pub(crate) async fn get_delegations(pubkey: &str) -> Json<Vec<Delegation>> {
    Json(get_single_mixnode_delegations(pubkey).await)
}

#[openapi(tag = "mix_node")]
#[get("/<pubkey>/description")]
pub(crate) async fn get_description(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Option<Json<NodeDescription>> {
    match state.inner.mixnode.clone().get_description(pubkey).await {
        Some(cache_value) => {
            trace!("Returning cached value for {}", pubkey);
            Some(Json(cache_value))
        }
        None => {
            trace!("No valid cache value for {}", pubkey);
            match state.inner.get_mix_node(pubkey).await {
                Some(bond) => {
                    match get_mix_node_description(
                        &bond.mix_node.host,
                        &bond.mix_node.http_api_port,
                    )
                    .await
                    {
                        Ok(response) => {
                            // cache the response and return as the HTTP response
                            state
                                .inner
                                .mixnode
                                .set_description(pubkey, response.clone())
                                .await;
                            Some(Json(response))
                        }
                        Err(e) => {
                            error!(
                                "Unable to get description for {} on {}:{} -> {}",
                                pubkey, bond.mix_node.host, bond.mix_node.http_api_port, e
                            );
                            Option::None
                        }
                    }
                }
                None => Option::None,
            }
        }
    }
}

#[openapi(tag = "mix_node")]
#[get("/<pubkey>/stats")]
pub(crate) async fn get_stats(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Option<Json<NodeStats>> {
    match state.inner.mixnode.get_node_stats(pubkey).await {
        Some(cache_value) => {
            trace!("Returning cached value for {}", pubkey);
            Some(Json(cache_value))
        }
        None => {
            trace!("No valid cache value for {}", pubkey);
            match state.inner.get_mix_node(pubkey).await {
                Some(bond) => {
                    match get_mix_node_stats(&bond.mix_node.host, &bond.mix_node.http_api_port)
                        .await
                    {
                        Ok(response) => {
                            // cache the response and return as the HTTP response
                            state
                                .inner
                                .mixnode
                                .set_node_stats(pubkey, response.clone())
                                .await;
                            Some(Json(response))
                        }
                        Err(e) => {
                            error!(
                                "Unable to get description for {} on {}:{} -> {}",
                                pubkey, bond.mix_node.host, bond.mix_node.http_api_port, e
                            );
                            Option::None
                        }
                    }
                }
                None => Option::None,
            }
        }
    }
}

#[openapi(tag = "mix_node")]
#[get("/<pubkey>/economic-dynamics-stats")]
pub(crate) async fn get_economic_dynamics_stats(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Option<Json<EconomicDynamicsStats>> {
    // if mixnode exists -> return fixture, otherwise return a None
    if state.inner.get_mix_node(pubkey).await.is_some() {
        Some(Json(EconomicDynamicsStats::dummy_fixture()))
    } else {
        None
    }
}

async fn get_mix_node_description(host: &str, port: &u16) -> Result<NodeDescription, ReqwestError> {
    reqwest::get(format!("http://{}:{}/description", host, port))
        .await?
        .json::<NodeDescription>()
        .await
}

async fn get_mix_node_stats(host: &str, port: &u16) -> Result<NodeStats, ReqwestError> {
    reqwest::get(format!("http://{}:{}/stats", host, port))
        .await?
        .json::<NodeStats>()
        .await
}
