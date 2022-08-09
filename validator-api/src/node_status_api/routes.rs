// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::helpers::{
    _compute_mixnode_reward_estimation, _get_mixnode_avg_uptime,
    _get_mixnode_inclusion_probability, _get_mixnode_reward_estimation,
    _get_mixnode_stake_saturation, _get_mixnode_status, _mixnode_core_status_count,
    _mixnode_report, _mixnode_uptime_history,
};
use crate::node_status_api::models::{
    ErrorResponse, GatewayStatusReport, GatewayUptimeHistory, MixnodeStatusReport,
    MixnodeUptimeHistory,
};
use crate::storage::ValidatorApiStorage;
use crate::ValidatorCache;
use mixnet_contract_common::NodeId;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use validator_api_requests::models::{
    ComputeRewardEstParam, GatewayCoreStatusResponse, InclusionProbabilityResponse,
    MixnodeCoreStatusResponse, MixnodeStatusResponse, RewardEstimationResponse,
    StakeSaturationResponse, UptimeResponse,
};

#[openapi(tag = "status")]
#[get("/gateway/<identity>/report")]
pub(crate) async fn gateway_report(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<GatewayStatusReport>, ErrorResponse> {
    storage
        .construct_gateway_report(identity)
        .await
        .map(Json)
        .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))
}

#[openapi(tag = "status")]
#[get("/gateway/<identity>/history")]
pub(crate) async fn gateway_uptime_history(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<GatewayUptimeHistory>, ErrorResponse> {
    storage
        .get_gateway_uptime_history(identity)
        .await
        .map(Json)
        .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))
}

#[openapi(tag = "status")]
#[get("/gateway/<identity>/core-status-count?<since>")]
pub(crate) async fn gateway_core_status_count(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
    since: Option<i64>,
) -> Json<GatewayCoreStatusResponse> {
    let count = storage
        .get_core_gateway_status_count(identity, since)
        .await
        .unwrap_or_default();

    Json(GatewayCoreStatusResponse {
        identity: identity.to_string(),
        count,
    })
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/report")]
pub(crate) async fn mixnode_report(
    storage: &State<ValidatorApiStorage>,
    mix_id: NodeId,
) -> Result<Json<MixnodeStatusReport>, ErrorResponse> {
    Ok(Json(_mixnode_report(storage, mix_id).await?))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/history")]
pub(crate) async fn mixnode_uptime_history(
    storage: &State<ValidatorApiStorage>,
    mix_id: NodeId,
) -> Result<Json<MixnodeUptimeHistory>, ErrorResponse> {
    Ok(Json(_mixnode_uptime_history(storage, mix_id).await?))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/core-status-count?<since>")]
pub(crate) async fn mixnode_core_status_count(
    storage: &State<ValidatorApiStorage>,
    mix_id: NodeId,
    since: Option<i64>,
) -> Result<Json<MixnodeCoreStatusResponse>, ErrorResponse> {
    Ok(Json(
        _mixnode_core_status_count(storage, mix_id, since).await?,
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/status")]
pub(crate) async fn get_mixnode_status(
    cache: &State<ValidatorCache>,
    mix_id: NodeId,
) -> Json<MixnodeStatusResponse> {
    Json(_get_mixnode_status(cache, mix_id).await)
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/reward-estimation")]
pub(crate) async fn get_mixnode_reward_estimation(
    cache: &State<ValidatorCache>,
    mix_id: NodeId,
) -> Result<Json<RewardEstimationResponse>, ErrorResponse> {
    Ok(Json(_get_mixnode_reward_estimation(cache, mix_id).await?))
}

#[openapi(tag = "status")]
#[post(
    "/mixnode/<mix_id>/compute-reward-estimation",
    data = "<user_reward_param>"
)]
pub(crate) async fn compute_mixnode_reward_estimation(
    user_reward_param: Json<ComputeRewardEstParam>,
    cache: &State<ValidatorCache>,
    mix_id: NodeId,
) -> Result<Json<RewardEstimationResponse>, ErrorResponse> {
    Ok(Json(
        _compute_mixnode_reward_estimation(user_reward_param.into_inner(), cache, mix_id).await?,
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/stake-saturation")]
pub(crate) async fn get_mixnode_stake_saturation(
    cache: &State<ValidatorCache>,
    mix_id: NodeId,
) -> Result<Json<StakeSaturationResponse>, ErrorResponse> {
    Ok(Json(_get_mixnode_stake_saturation(cache, mix_id).await?))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/inclusion-probability")]
pub(crate) async fn get_mixnode_inclusion_probability(
    cache: &State<ValidatorCache>,
    mix_id: NodeId,
) -> Result<Json<InclusionProbabilityResponse>, ErrorResponse> {
    Ok(Json(
        _get_mixnode_inclusion_probability(cache, mix_id).await?,
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/avg_uptime")]
pub(crate) async fn get_mixnode_avg_uptime(
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
    mix_id: NodeId,
) -> Result<Json<UptimeResponse>, ErrorResponse> {
    Ok(Json(_get_mixnode_avg_uptime(cache, storage, mix_id).await?))
}
