// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_node::models::{NodeDescription, NodeStats};
use crate::mix_nodes::delegations::{get_mixnode_delegations, get_single_mixnode_delegations};
use crate::state::ExplorerApiStateContext;
use mixnet_contract::Delegation;
use reqwest::Error as ReqwestError;
use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;

pub fn mix_node_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        settings: get_delegations,
        get_all_delegations,
        get_description,
        get_stats,
    ]
}

#[openapi(tag = "mix_node")]
#[get("/<pubkey>/delegations")]
pub(crate) async fn get_delegations(pubkey: &str) -> Json<Vec<Delegation>> {
    Json(get_single_mixnode_delegations(pubkey).await)
}

#[openapi(tag = "mix_node")]
#[get("/all_mix_delegations")]
pub(crate) async fn get_all_delegations() -> Json<Vec<Delegation>> {
    Json(get_mixnode_delegations().await)
}

#[openapi(tag = "mix_node")]
#[get("/<pubkey>/description")]
pub(crate) async fn get_description(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Option<Json<NodeDescription>> {
    match state
        .inner
        .mix_node_cache
        .clone()
        .get_description(pubkey)
        .await
    {
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
                                .mix_node_cache
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
    match state.inner.mix_node_cache.get_node_stats(pubkey).await {
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
                                .mix_node_cache
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
