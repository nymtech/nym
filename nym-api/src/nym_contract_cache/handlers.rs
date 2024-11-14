// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::helpers::{
    _get_active_set_legacy_mixnodes_detailed, _get_legacy_mixnodes_detailed,
    _get_rewarded_set_legacy_mixnodes_detailed,
};
use crate::support::http::state::AppState;
use crate::support::legacy_helpers::{to_legacy_gateway, to_legacy_mixnode};
use axum::extract::State;
use axum::{Json, Router};
use nym_api_requests::legacy::LegacyMixNodeDetailsWithLayer;
use nym_api_requests::models::MixNodeBondAnnotated;
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::{reward_params::RewardingParams, GatewayBond, Interval, NodeId};
use std::collections::HashSet;

// we want to mark the routes as deprecated in swagger, but still expose them
#[allow(deprecated)]
pub(crate) fn nym_contract_cache_routes() -> Router<AppState> {
    Router::new()
        .route("/mixnodes", axum::routing::get(get_mixnodes))
        .route(
            "/mixnodes/detailed",
            axum::routing::get(get_mixnodes_detailed),
        )
        .route("/gateways", axum::routing::get(get_gateways))
        .route("/mixnodes/rewarded", axum::routing::get(get_rewarded_set))
        .route(
            "/mixnodes/rewarded/detailed",
            axum::routing::get(get_rewarded_set_detailed),
        )
        .route("/mixnodes/active", axum::routing::get(get_active_set))
        .route(
            "/mixnodes/active/detailed",
            axum::routing::get(get_active_set_detailed),
        )
        .route(
            "/mixnodes/blacklisted",
            axum::routing::get(get_blacklisted_mixnodes),
        )
        .route(
            "/gateways/blacklisted",
            axum::routing::get(get_blacklisted_gateways),
        )
        .route(
            "/epoch/reward_params",
            axum::routing::get(get_interval_reward_params),
        )
        .route("/epoch/current", axum::routing::get(get_current_epoch))
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/mixnodes",
    responses(
        (status = 200, body = Vec<LegacyMixNodeDetailsWithLayer>)
    )
)]
#[deprecated]
async fn get_mixnodes(State(state): State<AppState>) -> Json<Vec<LegacyMixNodeDetailsWithLayer>> {
    let mut out = state.nym_contract_cache().legacy_mixnodes_filtered().await;

    let Ok(describe_cache) = state.described_nodes_cache.get().await else {
        return Json(out);
    };

    let Some(migrated_nymnodes) = state.nym_contract_cache().all_cached_nym_nodes().await else {
        return Json(out);
    };

    let Ok(annotations) = state.node_annotations().await else {
        return Json(out);
    };

    // safety: valid percentage value
    #[allow(clippy::unwrap_used)]
    let p50 = Performance::from_percentage_value(50).unwrap();

    for nym_node in &**migrated_nymnodes {
        // if we can't get it self-described data, ignore it
        let Some(description) = describe_cache.get_description(&nym_node.node_id()) else {
            continue;
        };
        // if the node hasn't declared it can be a mixnode, ignore it
        if !description.declared_role.mixnode {
            continue;
        }
        // if we don't have annotation for this node, ignore it
        let Some(annotation) = annotations.get(&nym_node.node_id()) else {
            continue;
        };
        // equivalent of legacy mixnode being blacklisted
        if annotation.last_24h_performance < p50 {
            continue;
        }

        let node = to_legacy_mixnode(nym_node, description);
        out.push(node);
    }

    Json(out)
}

// DEPRECATED: this endpoint now lives in `node_status_api`. Once all consumers are updated,
// replace this with
// ```
//  pub fn get_mixnodes_detailed() -> Redirect {
//      Redirect::to(uri!("/v1/status/mixnodes/detailed"))
//  }
// ```
#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/mixnodes/detailed",
    responses(
        (status = 200, body = Vec<MixNodeBondAnnotated>)
    )
)]
#[deprecated]
async fn get_mixnodes_detailed(State(state): State<AppState>) -> Json<Vec<MixNodeBondAnnotated>> {
    _get_legacy_mixnodes_detailed(state.node_status_cache())
        .await
        .into()
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/gateways",
    responses(
        (status = 200, body = Vec<GatewayBond>)
    )
)]
#[deprecated]
async fn get_gateways(State(state): State<AppState>) -> Json<Vec<GatewayBond>> {
    // legacy
    let mut out: Vec<GatewayBond> = state
        .nym_contract_cache()
        .legacy_gateways_filtered()
        .await
        .into_iter()
        .map(Into::into)
        .collect();

    let Ok(describe_cache) = state.described_nodes_cache.get().await else {
        return Json(out);
    };

    let Some(migrated_nymnodes) = state.nym_contract_cache().all_cached_nym_nodes().await else {
        return Json(out);
    };

    let Ok(annotations) = state.node_annotations().await else {
        return Json(out);
    };

    // safety: valid percentage value
    #[allow(clippy::unwrap_used)]
    let p50 = Performance::from_percentage_value(50).unwrap();

    for nym_node in &**migrated_nymnodes {
        // if we can't get it self-described data, ignore it
        let Some(description) = describe_cache.get_description(&nym_node.node_id()) else {
            continue;
        };
        // if the node hasn't declared it can be a gateway, ignore it
        if !description.declared_role.entry {
            continue;
        }
        // if we don't have annotation for this node, ignore it
        let Some(annotation) = annotations.get(&nym_node.node_id()) else {
            continue;
        };
        // equivalent of legacy gateway being blacklisted
        if annotation.last_24h_performance < p50 {
            continue;
        }

        let node = to_legacy_gateway(nym_node, description);
        out.push(node);
    }

    Json(out)
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/mixnodes/rewarded",
    responses(
        (status = 200, body = Vec<LegacyMixNodeDetailsWithLayer>)
    )
)]
#[deprecated]
async fn get_rewarded_set(
    State(state): State<AppState>,
) -> Json<Vec<LegacyMixNodeDetailsWithLayer>> {
    Json(
        state
            .nym_contract_cache()
            .legacy_v1_rewarded_set_mixnodes()
            .await
            .clone(),
    )
}

// DEPRECATED: this endpoint now lives in `node_status_api`. Once all consumers are updated,
// replace this with
// ```
//  pub fn get_mixnodes_set_detailed() -> Redirect {
//      Redirect::to(uri!("/v1/status/mixnodes/rewarded/detailed"))
//  }
// ```
#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/mixnodes/rewarded/detailed",
    responses(
        (status = 200, body = Vec<MixNodeBondAnnotated>)
    )
)]
#[deprecated]
async fn get_rewarded_set_detailed(
    State(state): State<AppState>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    _get_rewarded_set_legacy_mixnodes_detailed(
        state.node_status_cache(),
        state.nym_contract_cache(),
    )
    .await
    .into()
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/mixnodes/active",
    responses(
        (status = 200, body = Vec<LegacyMixNodeDetailsWithLayer>)
    )
)]
#[deprecated]
async fn get_active_set(State(state): State<AppState>) -> Json<Vec<LegacyMixNodeDetailsWithLayer>> {
    let mut out = state
        .nym_contract_cache()
        .legacy_v1_active_set_mixnodes()
        .await
        .clone();

    let Some(rewarded_set) = state.nym_contract_cache().rewarded_set().await else {
        return Json(out);
    };

    let Ok(describe_cache) = state.described_nodes_cache.get().await else {
        return Json(out);
    };

    let Some(migrated_nymnodes) = state.nym_contract_cache().all_cached_nym_nodes().await else {
        return Json(out);
    };

    let Ok(annotations) = state.node_annotations().await else {
        return Json(out);
    };

    // safety: valid percentage value
    #[allow(clippy::unwrap_used)]
    let p50 = Performance::from_percentage_value(50).unwrap();

    for nym_node in &**migrated_nymnodes {
        // if we can't get it self-described data, ignore it
        let Some(description) = describe_cache.get_description(&nym_node.node_id()) else {
            continue;
        };
        // if the node hasn't declared it can be a mixnode, ignore it
        if !description.declared_role.mixnode {
            continue;
        }
        // if we don't have annotation for this node, ignore it
        let Some(annotation) = annotations.get(&nym_node.node_id()) else {
            continue;
        };
        // equivalent of legacy mixnode being blacklisted
        if annotation.last_24h_performance < p50 {
            continue;
        }
        // if the node is not in the active set, ignore it
        if !rewarded_set.is_active_mixnode(&nym_node.node_id()) {
            continue;
        }

        let node = to_legacy_mixnode(nym_node, description);
        out.push(node);
    }

    Json(out)
}

// DEPRECATED: this endpoint now lives in `node_status_api`. Once all consumers are updated,
// replace this with
// ```
//  pub fn get_active_set_detailed() -> Redirect {
//      Redirect::to(uri!("/status/mixnodes/active/detailed"))
//  }
// ```

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/mixnodes/active/detailed",
    responses(
        (status = 200, body = Vec<MixNodeBondAnnotated>)
    )
)]
#[deprecated]
async fn get_active_set_detailed(State(state): State<AppState>) -> Json<Vec<MixNodeBondAnnotated>> {
    _get_active_set_legacy_mixnodes_detailed(state.node_status_cache(), state.nym_contract_cache())
        .await
        .into()
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/mixnodes/blacklisted",
    responses(
        (status = 200, body = Option<HashSet<NodeId>>)
    )
)]
#[deprecated]
async fn get_blacklisted_mixnodes(State(state): State<AppState>) -> Json<Option<HashSet<NodeId>>> {
    let blacklist = state
        .nym_contract_cache()
        .mixnodes_blacklist()
        .await
        .to_owned();
    if blacklist.is_empty() {
        None
    } else {
        Some(blacklist)
    }
    .into()
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/gateways/blacklisted",
    responses(
        (status = 200, body = Option<HashSet<String>>)
    )
)]
#[deprecated]
async fn get_blacklisted_gateways(State(state): State<AppState>) -> Json<Option<HashSet<String>>> {
    let cache = state.nym_contract_cache();
    let blacklist = cache.gateways_blacklist().await.clone();
    if blacklist.is_empty() {
        Json(None)
    } else {
        let gateways = cache.legacy_gateways_all().await;
        Json(Some(
            gateways
                .into_iter()
                .filter(|g| blacklist.contains(&g.node_id))
                .map(|g| g.gateway.identity_key.clone())
                .collect(),
        ))
    }
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/epoch/reward_params",
    responses(
        (status = 200, body = Option<RewardingParams>)
    )
)]
async fn get_interval_reward_params(
    State(state): State<AppState>,
) -> Json<Option<RewardingParams>> {
    state
        .nym_contract_cache()
        .interval_reward_params()
        .await
        .to_owned()
        .into()
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/epoch/current",
    responses(
        (status = 200, body = Option<Interval>)
    )
)]
async fn get_current_epoch(State(state): State<AppState>) -> Json<Option<Interval>> {
    state
        .nym_contract_cache()
        .current_interval()
        .await
        .to_owned()
        .into()
}
