// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// due to the macro expansion of rather old rocket macros...
#![allow(unused_imports)]

use super::helpers::_get_gateways_detailed;
use super::NodeStatusCache;
use crate::node_status_api::helpers::{
    _compute_mixnode_reward_estimation, _gateway_core_status_count, _gateway_report,
    _gateway_uptime_history, _get_active_set_detailed, _get_gateway_avg_uptime,
    _get_gateways_detailed_unfiltered, _get_mixnode_avg_uptime,
    _get_mixnode_inclusion_probabilities, _get_mixnode_inclusion_probability,
    _get_mixnode_reward_estimation, _get_mixnode_stake_saturation, _get_mixnode_status,
    _get_mixnodes_detailed, _get_mixnodes_detailed_unfiltered, _get_rewarded_set_detailed,
    _mixnode_core_status_count, _mixnode_report, _mixnode_uptime_history,
};
use crate::node_status_api::models::ErrorResponse;
use crate::storage::NymApiStorage;
use crate::NymContractCache;
use nym_api_requests::models::{
    AllInclusionProbabilitiesResponse, ComputeRewardEstParam, GatewayBondAnnotated,
    GatewayCoreStatusResponse, GatewayStatusReportResponse, GatewayUptimeHistoryResponse,
    GatewayUptimeResponse, InclusionProbabilityResponse, MixNodeBondAnnotated,
    MixnodeCoreStatusResponse, MixnodeStatusReportResponse, MixnodeStatusResponse,
    MixnodeUptimeHistoryResponse, RewardEstimationResponse, StakeSaturationResponse,
    UptimeResponse,
};
use nym_mixnet_contract_common::MixId;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;

#[openapi(tag = "status")]
#[get("/gateway/<identity>/report")]
pub(crate) async fn gateway_report(
    cache: &State<NodeStatusCache>,
    identity: &str,
) -> Result<Json<GatewayStatusReportResponse>, ErrorResponse> {
    Ok(Json(_gateway_report(cache, identity).await?))
}

#[openapi(tag = "status")]
#[get("/gateway/<identity>/history")]
pub(crate) async fn gateway_uptime_history(
    storage: &State<NymApiStorage>,
    identity: &str,
) -> Result<Json<GatewayUptimeHistoryResponse>, ErrorResponse> {
    Ok(Json(_gateway_uptime_history(storage, identity).await?))
}

#[openapi(tag = "status")]
#[get("/gateway/<identity>/core-status-count?<since>")]
pub(crate) async fn gateway_core_status_count(
    storage: &State<NymApiStorage>,
    identity: &str,
    since: Option<i64>,
) -> Result<Json<GatewayCoreStatusResponse>, ErrorResponse> {
    Ok(Json(
        _gateway_core_status_count(storage, identity, since).await?,
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/report")]
pub(crate) async fn mixnode_report(
    cache: &State<NodeStatusCache>,
    mix_id: MixId,
) -> Result<Json<MixnodeStatusReportResponse>, ErrorResponse> {
    Ok(Json(_mixnode_report(cache, mix_id).await?))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/history")]
pub(crate) async fn mixnode_uptime_history(
    storage: &State<NymApiStorage>,
    mix_id: MixId,
) -> Result<Json<MixnodeUptimeHistoryResponse>, ErrorResponse> {
    Ok(Json(_mixnode_uptime_history(storage, mix_id).await?))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/core-status-count?<since>")]
pub(crate) async fn mixnode_core_status_count(
    storage: &State<NymApiStorage>,
    mix_id: MixId,
    since: Option<i64>,
) -> Result<Json<MixnodeCoreStatusResponse>, ErrorResponse> {
    Ok(Json(
        _mixnode_core_status_count(storage, mix_id, since).await?,
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/status")]
pub(crate) async fn get_mixnode_status(
    cache: &State<NymContractCache>,
    mix_id: MixId,
) -> Json<MixnodeStatusResponse> {
    Json(_get_mixnode_status(cache, mix_id).await)
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/reward-estimation")]
pub(crate) async fn get_mixnode_reward_estimation(
    cache: &State<NodeStatusCache>,
    validator_cache: &State<NymContractCache>,
    mix_id: MixId,
) -> Result<Json<RewardEstimationResponse>, ErrorResponse> {
    Ok(Json(
        _get_mixnode_reward_estimation(cache, validator_cache, mix_id).await?,
    ))
}

#[openapi(tag = "status")]
#[post(
    "/mixnode/<mix_id>/compute-reward-estimation",
    data = "<user_reward_param>"
)]
pub(crate) async fn compute_mixnode_reward_estimation(
    user_reward_param: Json<ComputeRewardEstParam>,
    cache: &State<NodeStatusCache>,
    validator_cache: &State<NymContractCache>,
    mix_id: MixId,
) -> Result<Json<RewardEstimationResponse>, ErrorResponse> {
    Ok(Json(
        _compute_mixnode_reward_estimation(
            user_reward_param.into_inner(),
            cache,
            validator_cache,
            mix_id,
        )
        .await?,
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/stake-saturation")]
pub(crate) async fn get_mixnode_stake_saturation(
    cache: &State<NodeStatusCache>,
    validator_cache: &State<NymContractCache>,
    mix_id: MixId,
) -> Result<Json<StakeSaturationResponse>, ErrorResponse> {
    Ok(Json(
        _get_mixnode_stake_saturation(cache, validator_cache, mix_id).await?,
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/inclusion-probability")]
pub(crate) async fn get_mixnode_inclusion_probability(
    cache: &State<NodeStatusCache>,
    mix_id: MixId,
) -> Result<Json<InclusionProbabilityResponse>, ErrorResponse> {
    Ok(Json(
        _get_mixnode_inclusion_probability(cache, mix_id).await?,
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/avg_uptime")]
pub(crate) async fn get_mixnode_avg_uptime(
    cache: &State<NodeStatusCache>,
    mix_id: MixId,
) -> Result<Json<UptimeResponse>, ErrorResponse> {
    Ok(Json(_get_mixnode_avg_uptime(cache, mix_id).await?))
}

#[openapi(tag = "status")]
#[get("/gateway/<identity>/avg_uptime")]
pub(crate) async fn get_gateway_avg_uptime(
    cache: &State<NodeStatusCache>,
    identity: &str,
) -> Result<Json<GatewayUptimeResponse>, ErrorResponse> {
    Ok(Json(_get_gateway_avg_uptime(cache, identity).await?))
}

#[openapi(tag = "status")]
#[get("/mixnodes/inclusion_probability")]
pub(crate) async fn get_mixnode_inclusion_probabilities(
    cache: &State<NodeStatusCache>,
) -> Result<Json<AllInclusionProbabilitiesResponse>, ErrorResponse> {
    Ok(Json(_get_mixnode_inclusion_probabilities(cache).await?))
}

#[openapi(tag = "status")]
#[get("/mixnodes/detailed")]
pub async fn get_mixnodes_detailed(
    cache: &State<NodeStatusCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_mixnodes_detailed(cache).await)
}

#[openapi(tag = "status")]
#[get("/mixnodes/detailed-unfiltered")]
pub async fn get_mixnodes_detailed_unfiltered(
    cache: &State<NodeStatusCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_mixnodes_detailed_unfiltered(cache).await)
}

#[openapi(tag = "status")]
#[get("/mixnodes/rewarded/detailed")]
pub async fn get_rewarded_set_detailed(
    cache: &State<NodeStatusCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_rewarded_set_detailed(cache).await)
}

#[openapi(tag = "status")]
#[get("/mixnodes/active/detailed")]
pub async fn get_active_set_detailed(
    cache: &State<NodeStatusCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_active_set_detailed(cache).await)
}

#[openapi(tag = "status")]
#[get("/gateways/detailed")]
pub async fn get_gateways_detailed(
    cache: &State<NodeStatusCache>,
) -> Json<Vec<GatewayBondAnnotated>> {
    Json(_get_gateways_detailed(cache).await)
}

#[openapi(tag = "status")]
#[get("/gateways/detailed-unfiltered")]
pub async fn get_gateways_detailed_unfiltered(
    cache: &State<NodeStatusCache>,
) -> Json<Vec<GatewayBondAnnotated>> {
    Json(_get_gateways_detailed_unfiltered(cache).await)
}
