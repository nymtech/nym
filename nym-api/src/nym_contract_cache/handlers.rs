// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    node_status_api::helpers_deprecated::{
        _get_active_set_detailed, _get_mixnodes_detailed, _get_rewarded_set_detailed,
    },
    v2::AxumAppState,
};
use axum::{extract, Router};
use nym_api_requests::models::MixNodeBondAnnotated;
use nym_mixnet_contract_common::{
    mixnode::MixNodeDetails, reward_params::RewardingParams, GatewayBond, Interval, MixId,
};
use std::collections::HashSet;

pub(crate) fn nym_contract_cache_routes() -> Router<AxumAppState> {
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
        (status = 200, body = Vec<MixNodeDetails>)
    )
)]
async fn get_mixnodes(
    extract::State(state): extract::State<AxumAppState>,
) -> axum::Json<Vec<MixNodeDetails>> {
    state.nym_contract_cache().mixnodes_filtered().await.into()
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
async fn get_mixnodes_detailed(
    extract::State(state): extract::State<AxumAppState>,
) -> axum::Json<Vec<MixNodeBondAnnotated>> {
    _get_mixnodes_detailed(state.node_status_cache())
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
async fn get_gateways(
    extract::State(state): extract::State<AxumAppState>,
) -> axum::Json<Vec<GatewayBond>> {
    state.nym_contract_cache().gateways_filtered().await.into()
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/mixnodes/rewarded",
    responses(
        (status = 200, body = Vec<MixNodeDetails>)
    )
)]
async fn get_rewarded_set(
    extract::State(state): extract::State<AxumAppState>,
) -> axum::Json<Vec<MixNodeDetails>> {
    state
        .nym_contract_cache()
        .rewarded_set()
        .await
        .to_owned()
        .into()
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
async fn get_rewarded_set_detailed(
    extract::State(state): extract::State<AxumAppState>,
) -> axum::Json<Vec<MixNodeBondAnnotated>> {
    _get_rewarded_set_detailed(state.node_status_cache())
        .await
        .into()
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/mixnodes/active",
    responses(
        (status = 200, body = Vec<MixNodeDetails>)
    )
)]
async fn get_active_set(
    extract::State(state): extract::State<AxumAppState>,
) -> axum::Json<Vec<MixNodeDetails>> {
    state
        .nym_contract_cache()
        .active_set()
        .await
        .to_owned()
        .into()
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
async fn get_active_set_detailed(
    extract::State(state): extract::State<AxumAppState>,
) -> axum::Json<Vec<MixNodeBondAnnotated>> {
    _get_active_set_detailed(state.node_status_cache())
        .await
        .into()
}

#[utoipa::path(
    tag = "contract-cache",
    get,
    path = "/v1/mixnodes/blacklisted",
    responses(
        (status = 200, body = Option<HashSet<MixId>>)
    )
)]
async fn get_blacklisted_mixnodes(
    extract::State(state): extract::State<AxumAppState>,
) -> axum::Json<Option<HashSet<MixId>>> {
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
async fn get_blacklisted_gateways(
    extract::State(state): extract::State<AxumAppState>,
) -> axum::Json<Option<HashSet<String>>> {
    let blacklist = state
        .nym_contract_cache()
        .gateways_blacklist()
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
    path = "/v1/epoch/reward_params",
    responses(
        (status = 200, body = Option<RewardingParams>)
    )
)]
async fn get_interval_reward_params(
    extract::State(state): extract::State<AxumAppState>,
) -> axum::Json<Option<RewardingParams>> {
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
async fn get_current_epoch(
    extract::State(state): extract::State<AxumAppState>,
) -> axum::Json<Option<Interval>> {
    state
        .nym_contract_cache()
        .current_interval()
        .await
        .to_owned()
        .into()
}
