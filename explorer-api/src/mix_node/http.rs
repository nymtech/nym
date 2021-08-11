use reqwest::Error as ReqwestError;
use rocket::serde::json::Json;
use rocket::{Route, State};
use serde::Serialize;

use mixnet_contract::{Addr, Coin, Layer, MixNode};

use crate::mix_node::models::{NodeDescription, NodeStats};
use crate::mix_nodes::Location;
use crate::state::ExplorerApiStateContext;

pub fn mix_node_make_default_routes() -> Vec<Route> {
    routes_with_openapi![get_description, get_stats, list]
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub(crate) struct PrettyMixNodeBondWithLocation {
    pub location: Option<Location>,
    pub bond_amount: Coin,
    pub total_delegation: Coin,
    pub owner: Addr,
    pub layer: Layer,
    pub mix_node: MixNode,
}

#[openapi(tag = "mix_node")]
#[get("/")]
pub(crate) async fn list(
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<PrettyMixNodeBondWithLocation>> {
    Json(
        state
            .inner
            .mix_nodes
            .get()
            .await
            .value
            .values()
            .map(|i| {
                let mix_node = i.bond.clone();
                PrettyMixNodeBondWithLocation {
                    location: i.location.clone(),
                    bond_amount: mix_node.bond_amount,
                    total_delegation: mix_node.total_delegation,
                    owner: mix_node.owner,
                    layer: mix_node.layer,
                    mix_node: mix_node.mix_node,
                }
            })
            .collect::<Vec<PrettyMixNodeBondWithLocation>>(),
    )
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
