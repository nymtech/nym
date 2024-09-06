// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::handlers::MixIdParam;
use crate::node_status_api::helpers::{
    _get_active_set_legacy_mixnodes_detailed, _get_legacy_mixnodes_detailed,
    _get_mixnode_inclusion_probabilities, _get_mixnode_inclusion_probability,
    _get_mixnode_stake_saturation, _get_mixnode_status, _get_rewarded_set_legacy_mixnodes_detailed,
};
use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::state::AppState;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use nym_api_requests::models::{
    AllInclusionProbabilitiesResponse, InclusionProbabilityResponse, MixNodeBondAnnotated,
    MixnodeStatusResponse, StakeSaturationResponse,
};
use nym_mixnet_contract_common::NodeId;
use nym_types::monitoring::MonitorMessage;
use tracing::error;

// we want to mark the routes as deprecated in swagger, but still expose them
#[allow(deprecated)]
pub(super) fn mandatory_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/submit-gateway-monitoring-results",
            post(submit_gateway_monitoring_results),
        )
        .route(
            "/submit-node-monitoring-results",
            post(submit_node_monitoring_results),
        )
        .nest(
            "/mixnode/:mix_id",
            Router::new()
                .route("/status", get(get_mixnode_status))
                .route("/stake-saturation", get(get_mixnode_stake_saturation))
                .route(
                    "/inclusion-probability",
                    get(get_mixnode_inclusion_probability),
                ),
        )
        .merge(
            Router::new().nest(
                "/mixnodes",
                Router::new()
                    .route(
                        "/inclusion-probability",
                        get(get_mixnode_inclusion_probabilities),
                    )
                    .route("/detailed", get(get_mixnodes_detailed))
                    .route("/rewarded/detailed", get(get_rewarded_set_detailed))
                    .route("/active/detailed", get(get_active_set_detailed)),
            ),
        )
}

#[utoipa::path(
    tag = "status",
    post,
    path = "/v1/status/submit-gateway-monitoring-results",
    responses(
        (status = 200),
        (status = 400, body = ErrorResponse, description = "TBD"),
        (status = 403, body = ErrorResponse, description = "TBD"),
        (status = 500, body = ErrorResponse, description = "TBD"),
    ),
)]
pub(crate) async fn submit_gateway_monitoring_results(
    State(state): State<AxumAppState>,
    Json(message): Json<MonitorMessage>,
) -> AxumResult<()> {
    if !message.is_in_allowed() {
        return Err(AxumErrorResponse::forbidden(
            "Monitor not registered to submit results",
        ));
    }

    if !message.timely() {
        return Err(AxumErrorResponse::bad_request("Message is too old"));
    }

    if !message.verify() {
        return Err(AxumErrorResponse::bad_request("invalid signature"));
    }

    match state
        .storage
        .manager
        .submit_gateway_statuses_v2(message.results())
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => {
            error!("failed to submit gateway monitoring results: {err}");
            Err(AxumErrorResponse::internal_msg(
                "failed to submit gateway monitoring results",
            ))
        }
    }
}

#[utoipa::path(
    tag = "status",
    post,
    path = "/v1/status/submit-node-monitoring-results",
    responses(
        (status = 200),
        (status = 400, body = ErrorResponse, description = "TBD"),
        (status = 403, body = ErrorResponse, description = "TBD"),
        (status = 500, body = ErrorResponse, description = "TBD"),
    ),
)]
pub(crate) async fn submit_node_monitoring_results(
    State(state): State<AxumAppState>,
    Json(message): Json<MonitorMessage>,
) -> AxumResult<()> {
    if !message.is_in_allowed() {
        return Err(AxumErrorResponse::forbidden(
            "Monitor not registered to submit results",
        ));
    }

    if !message.timely() {
        return Err(AxumErrorResponse::bad_request("Message is too old"));
    }

    if !message.verify() {
        return Err(AxumErrorResponse::bad_request("invalid signature"));
    }

    match state
        .storage
        .manager
        .submit_mixnode_statuses_v2(message.results())
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => {
            error!("failed to submit node monitoring results: {err}");
            Err(AxumErrorResponse::internal_msg(
                "failed to submit node monitoring results",
            ))
        }
    }
}

#[utoipa::path(
    tag = "status",
    post,
    path = "/v1/status/submit-gateway-monitoring-results",
    responses(
        (status = 200),
        (status = 400, body = ErrorResponse, description = "TBD"),
        (status = 403, body = ErrorResponse, description = "TBD"),
        (status = 500, body = ErrorResponse, description = "TBD"),
    ),
)]
pub(crate) async fn submit_gateway_monitoring_results(
    State(state): State<AppState>,
    Json(message): Json<MonitorMessage>,
) -> AxumResult<()> {
    if !message.is_in_allowed() {
        return Err(AxumErrorResponse::forbidden(
            "Monitor not registered to submit results",
        ));
    }

    if !message.timely() {
        return Err(AxumErrorResponse::bad_request("Message is too old"));
    }

    if !message.verify() {
        return Err(AxumErrorResponse::bad_request("invalid signature"));
    }

    match state
        .storage
        .manager
        .submit_gateway_statuses_v2(message.results())
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => {
            error!("failed to submit gateway monitoring results: {err}");
            Err(AxumErrorResponse::internal_msg(
                "failed to submit gateway monitoring results",
            ))
        }
    }
}

#[utoipa::path(
    tag = "status",
    post,
    path = "/v1/status/submit-node-monitoring-results",
    responses(
        (status = 200),
        (status = 400, body = ErrorResponse, description = "TBD"),
        (status = 403, body = ErrorResponse, description = "TBD"),
        (status = 500, body = ErrorResponse, description = "TBD"),
    ),
)]
pub(crate) async fn submit_node_monitoring_results(
    State(state): State<AppState>,
    Json(message): Json<MonitorMessage>,
) -> AxumResult<()> {
    if !message.is_in_allowed() {
        return Err(AxumErrorResponse::forbidden(
            "Monitor not registered to submit results",
        ));
    }

    if !message.timely() {
        return Err(AxumErrorResponse::bad_request("Message is too old"));
    }

    if !message.verify() {
        return Err(AxumErrorResponse::bad_request("invalid signature"));
    }

    match state
        .storage
        .manager
        .submit_mixnode_statuses_v2(message.results())
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => {
            error!("failed to submit node monitoring results: {err}");
            Err(AxumErrorResponse::internal_msg(
                "failed to submit node monitoring results",
            ))
        }
    }
}

#[utoipa::path(
    tag = "status",
    get,
    params(
        MixIdParam
    ),
    path = "/v1/status/mixnode/{mix_id}/status",
    responses(
        (status = 200, body = MixnodeStatusResponse)
    )
)]
#[deprecated]
async fn get_mixnode_status(
    Path(MixIdParam { mix_id }): Path<MixIdParam>,
    State(state): State<AppState>,
) -> Json<MixnodeStatusResponse> {
    Json(_get_mixnode_status(state.nym_contract_cache(), mix_id).await)
}

#[utoipa::path(
    tag = "status",
    get,
    params(
        MixIdParam
    ),
    path = "/v1/status/mixnode/{mix_id}/stake-saturation",
    responses(
        (status = 200, body = StakeSaturationResponse)
    )
)]
#[deprecated]
async fn get_mixnode_stake_saturation(
    Path(mix_id): Path<NodeId>,
    State(state): State<AppState>,
) -> AxumResult<Json<StakeSaturationResponse>> {
    Ok(Json(
        _get_mixnode_stake_saturation(
            state.node_status_cache(),
            state.nym_contract_cache(),
            mix_id,
        )
        .await?,
    ))
}

#[utoipa::path(
    tag = "status",
    get,
    params(
        MixIdParam
    ),
    path = "/v1/status/mixnode/{mix_id}/inclusion-probability",
    responses(
        (status = 200, body = InclusionProbabilityResponse)
    )
)]
#[deprecated]
async fn get_mixnode_inclusion_probability(
    Path(mix_id): Path<NodeId>,
    State(state): State<AppState>,
) -> AxumResult<Json<InclusionProbabilityResponse>> {
    Ok(Json(
        _get_mixnode_inclusion_probability(state.node_status_cache(), mix_id).await?,
    ))
}

#[utoipa::path(
    tag = "status",
    get,
    path = "/v1/status/mixnodes/inclusion-probability",
    responses(
        (status = 200, body = AllInclusionProbabilitiesResponse)
    )
)]
#[deprecated]
async fn get_mixnode_inclusion_probabilities(
    State(state): State<AppState>,
) -> AxumResult<Json<AllInclusionProbabilitiesResponse>> {
    Ok(Json(
        _get_mixnode_inclusion_probabilities(state.node_status_cache()).await?,
    ))
}

#[utoipa::path(
    tag = "status",
    get,
    path = "/v1/status/mixnodes/detailed",
    responses(
        (status = 200, body = MixNodeBondAnnotated)
    )
)]
#[deprecated]
pub async fn get_mixnodes_detailed(
    State(state): State<AppState>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_legacy_mixnodes_detailed(state.node_status_cache()).await)
}

#[utoipa::path(
    tag = "status",
    get,
    path = "/v1/status/mixnodes/rewarded/detailed",
    responses(
        (status = 200, body = MixNodeBondAnnotated)
    )
)]
#[deprecated]
pub async fn get_rewarded_set_detailed(
    State(state): State<AppState>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(
        _get_rewarded_set_legacy_mixnodes_detailed(
            state.node_status_cache(),
            state.nym_contract_cache(),
        )
        .await,
    )
}

#[utoipa::path(
    tag = "status",
    get,
    path = "/v1/status/mixnodes/active/detailed",
    responses(
        (status = 200, body = MixNodeBondAnnotated)
    )
)]
#[deprecated]
pub async fn get_active_set_detailed(
    State(state): State<AppState>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(
        _get_active_set_legacy_mixnodes_detailed(
            state.node_status_cache(),
            state.nym_contract_cache(),
        )
        .await,
    )
}
