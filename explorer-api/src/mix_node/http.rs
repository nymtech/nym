// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_node::delegations::{
    get_single_mixnode_delegations, get_single_mixnode_delegations_summed,
};
use crate::mix_node::econ_stats::retrieve_mixnode_econ_stats;
use crate::mix_node::models::{
    EconomicDynamicsStats, NodeDescription, NodeStats, PrettyDetailedMixNodeBond, SummedDelegations,
};
use crate::mix_nodes;
use crate::state::ExplorerApiStateContext;
use mixnet_contract_common::{Delegation, NodeId};
use reqwest::Error as ReqwestError;
use rocket::response::status::NotFound;
use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;

pub fn mix_node_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        settings: get_delegations,
        get_delegations_summed,
        get_by_id,
        get_description,
        get_stats,
        get_economic_dynamics_stats,
        // =================================================
        // TO REMOVE ONCE OTHER PARTS OF THE SYSTEM MIGRATED
        // =================================================
        deprecated_get_delegations_by_identity,
        deprecated_get_delegations_summed_by_identity,
        deprecated_get_by_identity,
        deprecated_get_description_by_identity,
        deprecated_get_stats_by_identity,
        deprecated_get_economic_dynamics_stats_by_identity,
    ]
}

async fn get_mix_node_description(host: &str, port: u16) -> Result<NodeDescription, ReqwestError> {
    reqwest::get(format!("http://{}:{}/description", host, port))
        .await?
        .json::<NodeDescription>()
        .await
}

async fn get_mix_node_stats(host: &str, port: u16) -> Result<NodeStats, ReqwestError> {
    reqwest::get(format!("http://{}:{}/stats", host, port))
        .await?
        .json::<NodeStats>()
        .await
}

#[openapi(tag = "mix_nodes")]
#[get("/<mix_id>")]
pub(crate) async fn get_by_id(
    mix_id: NodeId,
    state: &State<ExplorerApiStateContext>,
) -> Result<Json<PrettyDetailedMixNodeBond>, NotFound<String>> {
    match state.inner.mixnodes.get_detailed_mixnode(mix_id).await {
        Some(mixnode) => Ok(Json(mixnode)),
        None => Err(NotFound("Mixnode not found".to_string())),
    }
}

#[openapi(tag = "mix_node")]
#[get("/<mix_id>/delegations")]
pub(crate) async fn get_delegations(
    mix_id: NodeId,
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<Delegation>> {
    Json(get_single_mixnode_delegations(&state.inner.validator_client, mix_id).await)
}

#[openapi(tag = "mix_node")]
#[get("/<mix_id>/delegations/summed")]
pub(crate) async fn get_delegations_summed(
    mix_id: NodeId,
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<SummedDelegations>> {
    Json(get_single_mixnode_delegations_summed(&state.inner.validator_client, mix_id).await)
}

#[openapi(tag = "mix_node")]
#[get("/<mix_id>/description")]
pub(crate) async fn get_description(
    mix_id: NodeId,
    state: &State<ExplorerApiStateContext>,
) -> Option<Json<NodeDescription>> {
    match state.inner.mixnode.clone().get_description(mix_id).await {
        Some(cache_value) => {
            trace!("Returning cached value for {}", mix_id);
            Some(Json(cache_value))
        }
        None => {
            trace!("No valid cache value for {}", mix_id);
            match state.inner.get_mix_node(mix_id).await {
                Some(bond) => {
                    match get_mix_node_description(
                        &bond.mix_node().host,
                        bond.mix_node().http_api_port,
                    )
                    .await
                    {
                        Ok(response) => {
                            // cache the response and return as the HTTP response
                            state
                                .inner
                                .mixnode
                                .set_description(mix_id, response.clone())
                                .await;
                            Some(Json(response))
                        }
                        Err(e) => {
                            error!(
                                "Unable to get description for {} on {}:{} -> {}",
                                mix_id,
                                bond.mix_node().host,
                                bond.mix_node().http_api_port,
                                e
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
#[get("/<mix_id>/stats")]
pub(crate) async fn get_stats(
    mix_id: NodeId,
    state: &State<ExplorerApiStateContext>,
) -> Option<Json<NodeStats>> {
    match state.inner.mixnode.get_node_stats(mix_id).await {
        Some(cache_value) => {
            trace!("Returning cached value for {}", mix_id);
            Some(Json(cache_value))
        }
        None => {
            trace!("No valid cache value for {}", mix_id);
            match state.inner.get_mix_node(mix_id).await {
                Some(bond) => {
                    match get_mix_node_stats(&bond.mix_node().host, bond.mix_node().http_api_port)
                        .await
                    {
                        Ok(response) => {
                            // cache the response and return as the HTTP response
                            state
                                .inner
                                .mixnode
                                .set_node_stats(mix_id, response.clone())
                                .await;
                            Some(Json(response))
                        }
                        Err(e) => {
                            error!(
                                "Unable to get description for {} on {}:{} -> {}",
                                mix_id,
                                bond.mix_node().host,
                                bond.mix_node().http_api_port,
                                e
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
#[get("/<mix_id>/economic-dynamics-stats")]
pub(crate) async fn get_economic_dynamics_stats(
    mix_id: NodeId,
    state: &State<ExplorerApiStateContext>,
) -> Option<Json<EconomicDynamicsStats>> {
    match state.inner.mixnode.get_econ_stats(mix_id).await {
        Some(cache_value) => {
            trace!("Returning cached value for {}", mix_id);
            Some(Json(cache_value))
        }
        None => {
            trace!("No valid cache value for {}", mix_id);

            // get fresh value from the validator API
            let econ_stats =
                retrieve_mixnode_econ_stats(&state.inner.validator_client, mix_id).await?;

            // update cache
            state
                .inner
                .mixnode
                .set_econ_stats(mix_id, econ_stats.clone())
                .await;
            Some(Json(econ_stats))
        }
    }
}

// =================================================
// TO REMOVE ONCE OTHER PARTS OF THE SYSTEM MIGRATED
// =================================================

#[openapi(tag = "mix_nodes")]
#[get("/deprecated/<pubkey>")]
pub(crate) async fn deprecated_get_by_identity(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Result<Json<PrettyDetailedMixNodeBond>, NotFound<String>> {
    let mix_id = match mix_nodes::helpers::best_effort_pubkey_to_mix_id(state, pubkey).await {
        Some(mix_id) => mix_id,
        None => {
            warn!(
                "there doesn't seem to exist a mixnode with identity {}",
                pubkey
            );
            return Err(NotFound("Mixnode not found".into()));
        }
    };
    get_by_id(mix_id, state).await
}

#[openapi(tag = "mix_node")]
#[get("/deprecated/<pubkey>/delegations")]
pub(crate) async fn deprecated_get_delegations_by_identity(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<Delegation>> {
    let mix_id = match mix_nodes::helpers::best_effort_pubkey_to_mix_id(state, pubkey).await {
        Some(mix_id) => mix_id,
        None => {
            warn!(
                "there doesn't seem to exist a mixnode with identity {}",
                pubkey
            );
            return Json(Vec::new());
        }
    };

    get_delegations(mix_id, state).await
}

#[openapi(tag = "mix_node")]
#[get("/deprecated/<pubkey>/delegations/summed")]
pub(crate) async fn deprecated_get_delegations_summed_by_identity(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<SummedDelegations>> {
    let mix_id = match mix_nodes::helpers::best_effort_pubkey_to_mix_id(state, pubkey).await {
        Some(mix_id) => mix_id,
        None => {
            warn!(
                "there doesn't seem to exist a mixnode with identity {}",
                pubkey
            );
            return Json(Vec::new());
        }
    };

    get_delegations_summed(mix_id, state).await
}

#[openapi(tag = "mix_node")]
#[get("/deprecated/<pubkey>/description")]
pub(crate) async fn deprecated_get_description_by_identity(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Option<Json<NodeDescription>> {
    let mix_id = match mix_nodes::helpers::best_effort_pubkey_to_mix_id(state, pubkey).await {
        Some(mix_id) => mix_id,
        None => {
            warn!(
                "there doesn't seem to exist a mixnode with identity {}",
                pubkey
            );
            return None;
        }
    };

    get_description(mix_id, state).await
}

#[openapi(tag = "mix_node")]
#[get("/deprecated/<pubkey>/stats")]
pub(crate) async fn deprecated_get_stats_by_identity(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Option<Json<NodeStats>> {
    let mix_id = match mix_nodes::helpers::best_effort_pubkey_to_mix_id(state, pubkey).await {
        Some(mix_id) => mix_id,
        None => {
            warn!(
                "there doesn't seem to exist a mixnode with identity {}",
                pubkey
            );
            return None;
        }
    };

    get_stats(mix_id, state).await
}

#[openapi(tag = "mix_node")]
#[get("/deprecated/<pubkey>/economic-dynamics-stats")]
pub(crate) async fn deprecated_get_economic_dynamics_stats_by_identity(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Option<Json<EconomicDynamicsStats>> {
    let mix_id = match mix_nodes::helpers::best_effort_pubkey_to_mix_id(state, pubkey).await {
        Some(mix_id) => mix_id,
        None => {
            warn!(
                "there doesn't seem to exist a mixnode with identity {}",
                pubkey
            );
            return None;
        }
    };

    get_economic_dynamics_stats(mix_id, state).await
}
