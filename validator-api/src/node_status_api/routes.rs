// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::models::Uptime;
use super::NodeStatusCache;
use crate::contract_cache::reward_estimate::compute_reward_estimate;
use crate::contract_cache::Cache;
use crate::node_status_api::models::{
    ErrorResponse, GatewayStatusReport, GatewayUptimeHistory, MixnodeStatusReport,
    MixnodeUptimeHistory,
};
use crate::storage::ValidatorApiStorage;
use crate::ValidatorCache;
use mixnet_contract_common::mixnode::MixNodeDetails;
use mixnet_contract_common::reward_params::{NodeRewardParams, Performance, RewardingParams};
use mixnet_contract_common::{Interval, MixNodeBond, NodeId};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use schemars::JsonSchema;
use serde::Deserialize;
use validator_api_requests::models::{
    AllInclusionProbabilitiesResponse, CoreNodeStatusResponse, DeprecatedRewardEstimationResponse,
    GatewayCoreStatusResponse, InclusionProbabilityResponse, MixnodeCoreStatusResponse,
    MixnodeStatusResponse, RewardEstimationResponse, StakeSaturationResponse, UptimeResponse,
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

async fn average_mixnode_uptime(
    mix_id: NodeId,
    current_epoch: Option<Interval>,
    storage: &State<ValidatorApiStorage>,
) -> Result<Uptime, ErrorResponse> {
    todo!()
    // Ok(if let Some(epoch) = current_epoch {
    //     storage
    //         .get_average_mixnode_uptime_in_the_last_24hrs(identity, epoch.end_unix_timestamp())
    //         .await
    //         .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))?
    // } else {
    //     Uptime::default()
    // })
}

pub(crate) async fn _mixnode_report(
    storage: &ValidatorApiStorage,
    mix_id: NodeId,
) -> Result<MixnodeStatusReport, ErrorResponse> {
    storage
        .construct_mixnode_report(mix_id)
        .await
        .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/report")]
pub(crate) async fn mixnode_report(
    storage: &State<ValidatorApiStorage>,
    mix_id: NodeId,
) -> Result<Json<MixnodeStatusReport>, ErrorResponse> {
    Ok(Json(_mixnode_report(storage, mix_id).await?))
}

pub(crate) async fn _mixnode_uptime_history(
    storage: &ValidatorApiStorage,
    mix_id: NodeId,
) -> Result<MixnodeUptimeHistory, ErrorResponse> {
    storage
        .get_mixnode_uptime_history(mix_id)
        .await
        .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/history")]
pub(crate) async fn mixnode_uptime_history(
    storage: &State<ValidatorApiStorage>,
    mix_id: NodeId,
) -> Result<Json<MixnodeUptimeHistory>, ErrorResponse> {
    Ok(Json(_mixnode_uptime_history(storage, mix_id).await?))
}

pub(crate) async fn _mixnode_core_status_count(
    storage: &State<ValidatorApiStorage>,
    mix_id: NodeId,
    since: Option<i64>,
) -> Result<MixnodeCoreStatusResponse, ErrorResponse> {
    let count = storage
        .get_core_mixnode_status_count(mix_id, since)
        .await
        .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))?;

    Ok(MixnodeCoreStatusResponse { mix_id, count })
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

pub(crate) async fn _get_mixnode_status(
    cache: &ValidatorCache,
    mix_id: NodeId,
) -> MixnodeStatusResponse {
    MixnodeStatusResponse {
        status: cache.mixnode_status(mix_id).await,
    }
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/status")]
pub(crate) async fn get_mixnode_status(
    cache: &State<ValidatorCache>,
    mix_id: NodeId,
) -> Json<MixnodeStatusResponse> {
    Json(_get_mixnode_status(cache, mix_id).await)
}

pub(crate) async fn _get_mixnode_reward_estimation(
    cache: &State<ValidatorCache>,
    mix_id: NodeId,
) -> Result<RewardEstimationResponse, ErrorResponse> {
    let (mixnode, status) = cache.mixnode_details(mix_id).await;
    if let Some(mixnode) = mixnode {
        let reward_params = cache.interval_reward_params().await;
        let as_at = reward_params.timestamp();
        let reward_params = reward_params
            .into_inner()
            .ok_or_else(|| ErrorResponse::new("server error", Status::InternalServerError))?;
        let current_interval = cache
            .current_interval()
            .await
            .into_inner()
            .ok_or_else(|| ErrorResponse::new("server error", Status::InternalServerError))?;

        let reward_estimation = compute_reward_estimate(
            &mixnode.mixnode_details,
            mixnode.performance,
            status.into(),
            reward_params,
            current_interval,
        );

        Ok(RewardEstimationResponse {
            estimation: reward_estimation,
            reward_params,
            epoch: current_interval,
            as_at,
        })
    } else {
        Err(ErrorResponse::new(
            "mixnode bond not found",
            Status::NotFound,
        ))
    }
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/reward-estimation")]
pub(crate) async fn get_mixnode_reward_estimation(
    cache: &State<ValidatorCache>,
    mix_id: NodeId,
) -> Result<Json<RewardEstimationResponse>, ErrorResponse> {
    Ok(Json(_get_mixnode_reward_estimation(cache, mix_id).await?))
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct ComputeRewardEstParam {
    performance: Option<Performance>,
    active_in_rewarded_set: Option<bool>,
    pledge_amount: Option<u64>,
    total_delegation: Option<u64>,
}

pub(crate) async fn _compute_mixnode_reward_estimation(
    user_reward_param: ComputeRewardEstParam,
    cache: &ValidatorCache,
    storage: &ValidatorApiStorage,
    mix_id: NodeId,
) -> Result<RewardEstimationResponse, ErrorResponse> {
    todo!()
}

#[openapi(tag = "status")]
#[post(
    "/mixnode/<mix_id>/compute-reward-estimation",
    data = "<user_reward_param>"
)]
pub(crate) async fn compute_mixnode_reward_estimation(
    user_reward_param: Json<ComputeRewardEstParam>,
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
    mix_id: NodeId,
) -> Result<Json<RewardEstimationResponse>, ErrorResponse> {
    let (bond, status) = cache.mixnode_details(&identity).await;
    if let Some(mut bond) = bond {
        let reward_params = cache.epoch_reward_params().await;
        let as_at = reward_params.timestamp();
        let reward_params = reward_params.into_inner();
        let base_operator_cost = cache.base_operator_cost().await.into_inner();

        let current_epoch = cache.current_epoch().await.into_inner();
        info!("{:?}", current_epoch);

        // For these parameters we either use the provided ones, or fall back to the system ones

        let uptime = if let Some(uptime) = user_reward_param.uptime {
            if uptime > 100 {
                return Err(ErrorResponse::new(
                    "Provided uptime out of bounds",
                    Status::UnprocessableEntity,
                ));
            }
            uptime
        } else {
            average_mixnode_uptime(&identity, current_epoch, storage)
                .await?
                .u8()
        };

        let is_active = user_reward_param
            .is_active
            .unwrap_or_else(|| status.is_active());

        if let Some(pledge_amount) = user_reward_param.pledge_amount {
            bond.mixnode_bond.original_pledge.amount = pledge_amount.into();
        }
        if let Some(total_delegation) = user_reward_param.total_delegation {
            bond.mixnode_bond.total_delegation.amount = total_delegation.into();
        }

        if bond.mixnode_bond.pledge_amount.amount.u128()
            + bond.mixnode_bond.total_delegation.amount.u128()
            > reward_params.staking_supply()
        {
            return Err(ErrorResponse::new(
                "Pledge plus delegation too large",
                Status::UnprocessableEntity,
            ));
        }

        let node_reward_params = NodeRewardParams::new(0, u128::from(uptime), is_active);
        let reward_params = RewardParams::new(reward_params, node_reward_params);

        estimate_reward(&bond.mixnode_bond, base_operator_cost, reward_params, as_at)
    } else {
        Err(ErrorResponse::new(
            "Mixnode bond not found",
            Status::NotFound,
        ))
    }
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/stake-saturation")]
pub(crate) async fn get_mixnode_stake_saturation(
    cache: &State<ValidatorCache>,
    mix_id: NodeId,
) -> Result<Json<StakeSaturationResponse>, ErrorResponse> {
    todo!()
    // let (bond, _) = cache.mixnode_details(&identity).await;
    // if let Some(bond) = bond {
    //     // Recompute the stake saturation just so that we can confidentaly state that the `as_at`
    //     // field is consistent and correct. Luckily this is very cheap.
    //     let interval_reward_params = cache.epoch_reward_params().await;
    //     let as_at = interval_reward_params.timestamp();
    //     let interval_reward_params = interval_reward_params.into_inner();
    //
    //     let saturation = bond.mixnode_bond.stake_saturation(
    //         interval_reward_params.staking_supply(),
    //         interval_reward_params.rewarded_set_size() as u32,
    //     );
    //
    //     Ok(Json(StakeSaturationResponse {
    //         saturation: saturation.to_num(),
    //         as_at,
    //     }))
    // } else {
    //     Err(ErrorResponse::new(
    //         "mixnode bond not found",
    //         Status::NotFound,
    //     ))
    // }
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/inclusion-probability")]
pub(crate) async fn get_mixnode_inclusion_probability(
    node_status_cache: &State<NodeStatusCache>,
    mix_id: NodeId,
) -> Json<Option<InclusionProbabilityResponse>> {
    node_status_cache
        .inclusion_probabilities()
        .await
        .map(Cache::into_inner)
        .and_then(|p| p.node(&identity).cloned())
        .map(|p| {
            Json(Some(InclusionProbabilityResponse {
                in_active: p.in_active.into(),
                in_reserve: p.in_reserve.into(),
            }))
        })
        .unwrap_or(Json(None))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/avg_uptime")]
pub(crate) async fn get_mixnode_avg_uptime(
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
    mix_id: NodeId,
) -> Result<Json<UptimeResponse>, ErrorResponse> {
    todo!()
    // let current_epoch = cache.current_epoch().await.into_inner();
    // let uptime = average_mixnode_uptime(&identity, current_epoch, storage).await?;
    //
    // Ok(Json(UptimeResponse {
    //     identity,
    //     avg_uptime: uptime.u8(),
    // }))
}

// DEPRECATED: the uptime is available as part of the `/mixnodes/detailed` endpoint
#[openapi(tag = "status")]
#[get("/mixnodes/avg_uptime")]
pub(crate) async fn get_mixnode_avg_uptimes(
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
) -> Result<Json<Vec<UptimeResponse>>, ErrorResponse> {
    todo!()
    // let mixnodes = cache.mixnodes().await;
    // let current_epoch = cache.current_epoch().await.into_inner();
    //
    // let mut response = Vec::new();
    // for mixnode in mixnodes {
    //     let uptime = average_mixnode_uptime(mixnode.identity(), current_epoch, storage).await?;
    //
    //     response.push(UptimeResponse {
    //         identity: mixnode.identity().to_string(),
    //         avg_uptime: uptime.u8(),
    //     })
    // }
    //
    // Ok(Json(response))
}

#[openapi(tag = "status")]
#[get("/mixnodes/inclusion_probability")]
pub(crate) async fn get_mixnode_inclusion_probabilities(
    cache: &State<NodeStatusCache>,
) -> Result<Json<AllInclusionProbabilitiesResponse>, ErrorResponse> {
    if let Some(prob) = cache.inclusion_probabilities().await {
        let as_at = prob.timestamp();
        let prob = prob.into_inner();
        Ok(Json(AllInclusionProbabilitiesResponse {
            inclusion_probabilities: prob.inclusion_probabilities,
            samples: prob.samples,
            elapsed: prob.elapsed,
            delta_max: prob.delta_max,
            delta_l2: prob.delta_l2,
            as_at,
        }))
    } else {
        Err(ErrorResponse::new(
            "No data available".to_string(),
            Status::ServiceUnavailable,
        ))
    }
}
