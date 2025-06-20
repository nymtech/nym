// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::helpers::{
    _get_active_set_legacy_mixnodes_detailed, _get_legacy_mixnodes_detailed,
    _get_rewarded_set_legacy_mixnodes_detailed,
};
use crate::node_status_api::models::ApiResult;
use crate::support::http::state::AppState;
use crate::support::legacy_helpers::{to_legacy_gateway, to_legacy_mixnode};
use axum::extract::{Query, State};
use axum::Router;
use nym_api_requests::legacy::LegacyMixNodeDetailsWithLayer;
use nym_api_requests::models::{KeyRotationInfoResponse, MixNodeBondAnnotated};
use nym_http_api_common::{FormattedResponse, OutputParams};
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
        .route(
            "/epoch/key-rotation-info",
            axum::routing::get(get_current_key_rotation_info),
        )
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/mixnodes",
    responses(
        (status = 200, content(
            (Vec<LegacyMixNodeDetailsWithLayer> = "application/json"),
            (Vec<LegacyMixNodeDetailsWithLayer> = "application/yaml"),
            (Vec<LegacyMixNodeDetailsWithLayer> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
async fn get_mixnodes(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Vec<LegacyMixNodeDetailsWithLayer>> {
    let output = output.output.unwrap_or_default();

    let Ok(describe_cache) = state.described_nodes_cache.get().await else {
        return output.to_response(Vec::new());
    };

    let Some(migrated_nymnodes) = state.nym_contract_cache().all_cached_nym_nodes().await else {
        return output.to_response(Vec::new());
    };

    let Ok(annotations) = state.node_annotations().await else {
        return output.to_response(Vec::new());
    };

    // safety: valid percentage value
    #[allow(clippy::unwrap_used)]
    let p50 = Performance::from_percentage_value(50).unwrap();

    let mut nodes = Vec::new();
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
        nodes.push(node);
    }

    output.to_response(nodes)
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
        (status = 200, content(
            (Vec<MixNodeBondAnnotated> = "application/json"),
            (Vec<MixNodeBondAnnotated> = "application/yaml"),
            (Vec<MixNodeBondAnnotated> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
async fn get_mixnodes_detailed(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Vec<MixNodeBondAnnotated>> {
    let output = output.output.unwrap_or_default();

    output.to_response(_get_legacy_mixnodes_detailed(state.node_status_cache()).await)
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/gateways",
    responses(
        (status = 200, content(
            (Vec<GatewayBond> = "application/json"),
            (Vec<GatewayBond> = "application/yaml"),
            (Vec<GatewayBond> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
async fn get_gateways(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Vec<GatewayBond>> {
    let output = output.output.unwrap_or_default();

    let mut nodes = Vec::new();

    let Ok(describe_cache) = state.described_nodes_cache.get().await else {
        return output.to_response(nodes);
    };

    let Some(migrated_nymnodes) = state.nym_contract_cache().all_cached_nym_nodes().await else {
        return output.to_response(nodes);
    };

    let Ok(annotations) = state.node_annotations().await else {
        return output.to_response(nodes);
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
        nodes.push(node);
    }

    output.to_response(nodes)
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/mixnodes/rewarded",
    responses(
        (status = 200, content(
            (Vec<LegacyMixNodeDetailsWithLayer> = "application/json"),
            (Vec<LegacyMixNodeDetailsWithLayer> = "application/yaml"),
            (Vec<LegacyMixNodeDetailsWithLayer> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
async fn get_rewarded_set(
    Query(output): Query<OutputParams>,
    State(_state): State<AppState>,
) -> FormattedResponse<Vec<LegacyMixNodeDetailsWithLayer>> {
    let output = output.output.unwrap_or_default();

    output.to_response(Vec::new())
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
        (status = 200, content(
            (Vec<MixNodeBondAnnotated> = "application/json"),
            (Vec<MixNodeBondAnnotated> = "application/yaml"),
            (Vec<MixNodeBondAnnotated> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
async fn get_rewarded_set_detailed(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Vec<MixNodeBondAnnotated>> {
    let output = output.output.unwrap_or_default();

    output.to_response(
        _get_rewarded_set_legacy_mixnodes_detailed(
            state.node_status_cache(),
            state.nym_contract_cache(),
        )
        .await,
    )
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/mixnodes/active",
    responses(
        (status = 200, content(
            (Vec<LegacyMixNodeDetailsWithLayer> = "application/json"),
            (Vec<LegacyMixNodeDetailsWithLayer> = "application/yaml"),
            (Vec<LegacyMixNodeDetailsWithLayer> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
async fn get_active_set(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Vec<LegacyMixNodeDetailsWithLayer>> {
    let output = output.output.unwrap_or_default();

    let mut out = Vec::new();

    let Some(rewarded_set) = state.nym_contract_cache().rewarded_set().await else {
        return output.to_response(out);
    };

    let Ok(describe_cache) = state.described_nodes_cache.get().await else {
        return output.to_response(out);
    };

    let Some(migrated_nymnodes) = state.nym_contract_cache().all_cached_nym_nodes().await else {
        return output.to_response(out);
    };

    let Ok(annotations) = state.node_annotations().await else {
        return output.to_response(out);
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

    output.to_response(out)
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
        (status = 200, content(
            (Vec<MixNodeBondAnnotated> = "application/json"),
            (Vec<MixNodeBondAnnotated> = "application/yaml"),
            (Vec<MixNodeBondAnnotated> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
async fn get_active_set_detailed(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Vec<MixNodeBondAnnotated>> {
    let output = output.output.unwrap_or_default();

    output.to_response(
        _get_active_set_legacy_mixnodes_detailed(
            state.node_status_cache(),
            state.nym_contract_cache(),
        )
        .await,
    )
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/mixnodes/blacklisted",
    responses(
        (status = 200, content(
            (Option<HashSet<NodeId>> = "application/json"),
            (Option<HashSet<NodeId>> = "application/yaml"),
            (Option<HashSet<NodeId>> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
async fn get_blacklisted_mixnodes(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Option<HashSet<NodeId>>> {
    let output = output.output.unwrap_or_default();

    let cache = state.nym_contract_cache();

    // since blacklist has been removed, the equivalent of a blacklisted node is a legacy node
    let mixnodes = cache.legacy_mixnodes_all().await;
    output.to_response(Some(mixnodes.into_iter().map(|m| m.mix_id()).collect()))
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/gateways/blacklisted",
    responses(
        (status = 200, content(
            (Option<HashSet<NodeId>> = "application/json"),
            (Option<HashSet<NodeId>> = "application/yaml"),
            (Option<HashSet<NodeId>> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
#[deprecated]
async fn get_blacklisted_gateways(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Option<HashSet<String>>> {
    let output = output.output.unwrap_or_default();

    let cache = state.nym_contract_cache();
    // since blacklist has been removed, the equivalent of a blacklisted node is a legacy node
    let gateways = cache.legacy_gateways_all().await;
    output.to_response(Some(
        gateways
            .into_iter()
            .map(|g| g.gateway.identity_key.clone())
            .collect(),
    ))
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/epoch/reward_params",
    responses(
        (status = 200, content(
            (Option<RewardingParams> = "application/json"),
            (Option<RewardingParams> = "application/yaml"),
            (Option<RewardingParams> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn get_interval_reward_params(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Option<RewardingParams>> {
    let output = output.output.unwrap_or_default();

    output.to_response(
        state
            .nym_contract_cache()
            .interval_reward_params()
            .await
            .ok(),
    )
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/epoch/current",
    responses(
        (status = 200, content(
            (Option<Interval> = "application/json"),
            (Option<Interval> = "application/yaml"),
            (Option<Interval> = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn get_current_epoch(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<Option<Interval>> {
    let output = output.output.unwrap_or_default();

    output.to_response(state.nym_contract_cache().current_interval().await.ok())
}

//
#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/key-rotation-info",
    context_path = "/v1/epoch",
    responses(
        (status = 200, content(
            (KeyRotationInfoResponse = "application/json"),
            (KeyRotationInfoResponse = "application/yaml"),
            (KeyRotationInfoResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn get_current_key_rotation_info(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> ApiResult<FormattedResponse<KeyRotationInfoResponse>> {
    let output = output.output.unwrap_or_default();

    let contract_cache = state.nym_contract_cache();
    let current_interval = contract_cache.current_interval().await?;
    let key_rotation_state = contract_cache.get_key_rotation_state().await?;

    Ok(output.to_response(KeyRotationInfoResponse {
        key_rotation_state,
        current_absolute_epoch_id: current_interval.current_epoch_absolute_id(),
        current_epoch_start: current_interval.current_epoch_start(),
        epoch_duration: current_interval.epoch_length(),
    }))
}
