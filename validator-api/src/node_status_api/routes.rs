// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{
    ErrorResponse, GatewayStatusReport, GatewayUptimeHistory, MixnodeStatusReport,
    MixnodeUptimeHistory,
};
use crate::storage::ValidatorApiStorage;
use crate::ValidatorCache;
use mixnet_contract_common::reward_params::{NodeRewardParams, RewardingParams};
use mixnet_contract_common::{Interval, MixNodeBond};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use schemars::JsonSchema;
use serde::Deserialize;
use validator_api_requests::models::{
    CoreNodeStatusResponse, InclusionProbabilityResponse, MixnodeStatusResponse,
    RewardEstimationResponse, StakeSaturationResponse, UptimeResponse,
};

use super::models::Uptime;

async fn average_mixnode_uptime(
    identity: &str,
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

fn estimate_reward(
    mixnode_bond: &MixNodeBond,
    base_operator_cost: u64,
    reward_params: RewardingParams,
    as_at: i64,
) -> Result<Json<RewardEstimationResponse>, ErrorResponse> {
    todo!()
    // match mixnode_bond.estimate_reward(base_operator_cost, &reward_params) {
    //     Ok(reward_estimate) => {
    //         let reponse = RewardEstimationResponse {
    //             estimated_total_node_reward: reward_estimate.total_node_reward,
    //             estimated_operator_reward: reward_estimate.operator_reward,
    //             estimated_delegators_reward: reward_estimate.delegators_reward,
    //             estimated_node_profit: reward_estimate.node_profit,
    //             estimated_operator_cost: reward_estimate.operator_cost,
    //             reward_params,
    //             as_at,
    //         };
    //         Ok(Json(reponse))
    //     }
    //     Err(e) => Err(ErrorResponse::new(
    //         e.to_string(),
    //         Status::InternalServerError,
    //     )),
    // }
}

#[openapi(tag = "status")]
#[get("/mixnode/<identity>/report")]
pub(crate) async fn mixnode_report(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<MixnodeStatusReport>, ErrorResponse> {
    storage
        .construct_mixnode_report(identity)
        .await
        .map(Json)
        .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))
}

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
#[get("/mixnode/<identity>/history")]
pub(crate) async fn mixnode_uptime_history(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<MixnodeUptimeHistory>, ErrorResponse> {
    storage
        .get_mixnode_uptime_history(identity)
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
#[get("/mixnode/<identity>/core-status-count?<since>")]
pub(crate) async fn mixnode_core_status_count(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
    since: Option<i64>,
) -> Json<CoreNodeStatusResponse> {
    let count = storage
        .get_core_mixnode_status_count(identity, since)
        .await
        .unwrap_or_default();

    Json(CoreNodeStatusResponse {
        identity: identity.to_string(),
        count,
    })
}

#[openapi(tag = "status")]
#[get("/gateway/<identity>/core-status-count?<since>")]
pub(crate) async fn gateway_core_status_count(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
    since: Option<i64>,
) -> Json<CoreNodeStatusResponse> {
    let count = storage
        .get_core_gateway_status_count(identity, since)
        .await
        .unwrap_or_default();

    Json(CoreNodeStatusResponse {
        identity: identity.to_string(),
        count,
    })
}

#[openapi(tag = "status")]
#[get("/mixnode/<identity>/status")]
pub(crate) async fn get_mixnode_status(
    cache: &State<ValidatorCache>,
    identity: String,
) -> Json<MixnodeStatusResponse> {
    Json(MixnodeStatusResponse {
        status: cache.mixnode_status(identity).await,
    })
}

#[openapi(tag = "status")]
#[get("/mixnode/<identity>/reward-estimation")]
pub(crate) async fn get_mixnode_reward_estimation(
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
    identity: String,
) -> Result<Json<RewardEstimationResponse>, ErrorResponse> {
    todo!()
    // let (bond, status) = cache.mixnode_details(&identity).await;
    // if let Some(bond) = bond {
    //     let reward_params = cache.epoch_reward_params().await;
    //     let as_at = reward_params.timestamp();
    //     let reward_params = reward_params.into_inner();
    //     let base_operator_cost = cache.base_operator_cost().await.into_inner();
    //
    //     let current_epoch = cache.current_epoch().await.into_inner();
    //     info!("{:?}", current_epoch);
    //
    //     let uptime = average_mixnode_uptime(&identity, current_epoch, storage)
    //         .await?
    //         .u8();
    //
    //     let node_reward_params = NodeRewardParams::new(0, u128::from(uptime), status.is_active());
    //     let reward_params = RewardParams::new(reward_params, node_reward_params);
    //
    //     estimate_reward(&bond.mixnode_bond, base_operator_cost, reward_params, as_at)
    // } else {
    //     Err(ErrorResponse::new(
    //         "mixnode bond not found",
    //         Status::NotFound,
    //     ))
    // }
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct ComputeRewardEstParam {
    uptime: Option<u8>,
    is_active: Option<bool>,
    pledge_amount: Option<u64>,
    total_delegation: Option<u64>,
}

#[openapi(tag = "status")]
#[post(
    "/mixnode/<identity>/compute-reward-estimation",
    data = "<user_reward_param>"
)]
pub(crate) async fn compute_mixnode_reward_estimation(
    user_reward_param: Json<ComputeRewardEstParam>,
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
    identity: String,
) -> Result<Json<RewardEstimationResponse>, ErrorResponse> {
    todo!()
    // let (bond, status) = cache.mixnode_details(&identity).await;
    // if let Some(mut bond) = bond {
    //     let reward_params = cache.epoch_reward_params().await;
    //     let as_at = reward_params.timestamp();
    //     let reward_params = reward_params.into_inner();
    //     let base_operator_cost = cache.base_operator_cost().await.into_inner();
    //
    //     let current_epoch = cache.current_epoch().await.into_inner();
    //     info!("{:?}", current_epoch);
    //
    //     // For these parameters we either use the provided ones, or fall back to the system ones
    //
    //     let uptime = if let Some(uptime) = user_reward_param.uptime {
    //         uptime
    //     } else {
    //         average_mixnode_uptime(&identity, current_epoch, storage)
    //             .await?
    //             .u8()
    //     };
    //
    //     let is_active = user_reward_param
    //         .is_active
    //         .unwrap_or_else(|| status.is_active());
    //
    //     if let Some(pledge_amount) = user_reward_param.pledge_amount {
    //         bond.mixnode_bond.original_pledge.amount = pledge_amount.into();
    //     }
    //     if let Some(total_delegation) = user_reward_param.total_delegation {
    //         bond.mixnode_bond.total_delegation.amount = total_delegation.into();
    //     }
    //
    //     let node_reward_params = NodeRewardParams::new(0, u128::from(uptime), is_active);
    //     let reward_params = RewardParams::new(reward_params, node_reward_params);
    //
    //     estimate_reward(&bond.mixnode_bond, base_operator_cost, reward_params, as_at)
    // } else {
    //     Err(ErrorResponse::new(
    //         "mixnode bond not found",
    //         Status::NotFound,
    //     ))
    // }
}

#[openapi(tag = "status")]
#[get("/mixnode/<identity>/stake-saturation")]
pub(crate) async fn get_mixnode_stake_saturation(
    cache: &State<ValidatorCache>,
    identity: String,
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
#[get("/mixnode/<identity>/inclusion-probability")]
pub(crate) async fn get_mixnode_inclusion_probability(
    cache: &State<ValidatorCache>,
    identity: String,
) -> Json<Option<InclusionProbabilityResponse>> {
    let mixnodes = cache.mixnodes().await;
    // let rewarding_params = cache.epoch_reward_params().await.into_inner();

    todo!()
    // if let Some(target_mixnode) = mixnodes.iter().find(|x| x.identity() == &identity) {
    //     let total_bonded_tokens = mixnodes
    //         .iter()
    //         .fold(0u128, |acc, x| acc + x.total_bond().unwrap_or_default())
    //         as f64;
    //
    //     let rewarded_set_size = rewarding_params.rewarded_set_size() as f64;
    //     let active_set_size = rewarding_params.active_set_size() as f64;
    //
    //     let prob_one_draw =
    //         target_mixnode.total_bond().unwrap_or_default() as f64 / total_bonded_tokens;
    //     // Chance to be selected in any draw for active set
    //     let prob_active_set = if mixnodes.len() <= active_set_size as usize {
    //         1.0
    //     } else {
    //         active_set_size * prob_one_draw
    //     };
    //     // This is likely slightly too high, as we're not correcting form them not being selected in active, should be chance to be selected, minus the chance for being not selected in reserve
    //     let prob_reserve_set = if mixnodes.len() <= rewarded_set_size as usize {
    //         1.0
    //     } else {
    //         (rewarded_set_size - active_set_size) * prob_one_draw
    //     };
    //
    //     Json(Some(InclusionProbabilityResponse {
    //         in_active: prob_active_set.into(),
    //         in_reserve: prob_reserve_set.into(),
    //     }))
    // } else {
    //     Json(None)
    // }
}

#[openapi(tag = "status")]
#[get("/mixnode/<identity>/avg_uptime")]
pub(crate) async fn get_mixnode_avg_uptime(
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
    identity: String,
) -> Result<Json<UptimeResponse>, ErrorResponse> {
    let current_epoch = cache.current_epoch().await.into_inner();
    let uptime = average_mixnode_uptime(&identity, current_epoch, storage).await?;

    Ok(Json(UptimeResponse {
        identity,
        avg_uptime: uptime.u8(),
    }))
}

// DEPRECATED: the uptime is available as part of the `/mixnodes/detailed` endpoint
#[openapi(tag = "status")]
#[get("/mixnodes/avg_uptime")]
pub(crate) async fn get_mixnode_avg_uptimes(
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
) -> Result<Json<Vec<UptimeResponse>>, ErrorResponse> {
    let mixnodes = cache.mixnodes().await;
    let current_epoch = cache.current_epoch().await.into_inner();

    let mut response = Vec::new();
    for mixnode in mixnodes {
        let uptime = average_mixnode_uptime(mixnode.identity(), current_epoch, storage).await?;

        response.push(UptimeResponse {
            identity: mixnode.identity().to_string(),
            avg_uptime: uptime.u8(),
        })
    }

    Ok(Json(response))
}
