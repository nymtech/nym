// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{
    ErrorResponse, GatewayStatusReport, GatewayUptimeHistory, MixnodeStatusReport,
    MixnodeUptimeHistory,
};
use crate::storage::ValidatorApiStorage;
use crate::ValidatorCache;
use mixnet_contract_common::reward_params::{NodeRewardParams, RewardParams};
use mixnet_contract_common::Interval;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use validator_api_requests::models::{
    CoreNodeStatusResponse, InclusionProbabilityResponse, MixnodeStatusResponse,
    RewardEstimationResponse, StakeSaturationResponse, UptimeResponse,
};

use super::models::Uptime;

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

#[get("/mixnode/<identity>/status")]
pub(crate) async fn get_mixnode_status(
    cache: &State<ValidatorCache>,
    identity: String,
) -> Json<MixnodeStatusResponse> {
    Json(MixnodeStatusResponse {
        status: cache.mixnode_status(identity).await,
    })
}

#[get("/mixnode/<identity>/reward-estimation")]
pub(crate) async fn get_mixnode_reward_estimation(
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
    identity: String,
) -> Result<Json<RewardEstimationResponse>, ErrorResponse> {
    let (bond, status) = cache.mixnode_details(&identity).await;
    if let Some(bond) = bond {
        let reward_params = cache.epoch_reward_params().await;
        let as_at = reward_params.timestamp();
        let reward_params = reward_params.into_inner();

        let current_epoch = cache.current_epoch().await.into_inner();
        info!("{:?}", current_epoch);

        let uptime = if let Some(epoch) = current_epoch {
            storage
                .get_average_mixnode_uptime_in_the_last_24hrs(&identity, epoch.end_unix_timestamp())
                .await
                .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))?
        } else {
            Uptime::default()
        };

        let node_reward_params = NodeRewardParams::new(0, uptime.u8() as u128, status.is_active());
        let reward_params = RewardParams::new(reward_params, node_reward_params);

        match bond.estimate_reward(&reward_params) {
            Ok((
                estimated_total_node_reward,
                estimated_operator_reward,
                estimated_delegators_reward,
            )) => {
                let reponse = RewardEstimationResponse {
                    estimated_total_node_reward,
                    estimated_operator_reward,
                    estimated_delegators_reward,
                    reward_params,
                    as_at,
                };
                Ok(Json(reponse))
            }
            Err(e) => Err(ErrorResponse::new(
                e.to_string(),
                Status::InternalServerError,
            )),
        }
    } else {
        Err(ErrorResponse::new(
            "mixnode bond not found",
            Status::NotFound,
        ))
    }
}

#[get("/mixnode/<identity>/stake-saturation")]
pub(crate) async fn get_mixnode_stake_saturation(
    cache: &State<ValidatorCache>,
    identity: String,
) -> Result<Json<StakeSaturationResponse>, ErrorResponse> {
    let (bond, _) = cache.mixnode_details(&identity).await;
    if let Some(bond) = bond {
        let interval_reward_params = cache.epoch_reward_params().await;
        let as_at = interval_reward_params.timestamp();
        let interval_reward_params = interval_reward_params.into_inner();

        let saturation = bond.stake_saturation(
            interval_reward_params.circulating_supply(),
            interval_reward_params.rewarded_set_size() as u32,
        );

        Ok(Json(StakeSaturationResponse {
            saturation: saturation.to_num(),
            as_at,
        }))
    } else {
        Err(ErrorResponse::new(
            "mixnode bond not found",
            Status::NotFound,
        ))
    }
}

#[get("/mixnode/<identity>/inclusion-probability")]
pub(crate) async fn get_mixnode_inclusion_probability(
    cache: &State<ValidatorCache>,
    identity: String,
) -> Json<Option<InclusionProbabilityResponse>> {
    let mixnodes = cache.mixnodes().await;

    if let Some(target_mixnode) = mixnodes.iter().find(|x| x.identity() == &identity) {
        let total_bonded_tokens = mixnodes
            .iter()
            .fold(0u128, |acc, x| acc + x.total_bond().unwrap_or_default())
            as f64;

        let rewarding_params = cache.epoch_reward_params().await.into_inner();
        let rewarded_set_size = rewarding_params.rewarded_set_size() as f64;
        let active_set_size = rewarding_params.active_set_size() as f64;

        let prob_one_draw =
            target_mixnode.total_bond().unwrap_or_default() as f64 / total_bonded_tokens;
        // Chance to be selected in any draw for active set
        let prob_active_set = active_set_size * prob_one_draw;
        // This is likely slightly too high, as we're not correcting form them not being selected in active, should be chance to be selected, minus the chance for being not selected in reserve
        let prob_reserve_set = (rewarded_set_size - active_set_size) * prob_one_draw;
        // (rewarded_set_size - active_set_size) * prob_one_draw * (1. - prob_active_set);

        Json(Some(InclusionProbabilityResponse {
            in_active: if prob_active_set > 1. {
                1.
            } else {
                prob_active_set
            } as f32,
            in_reserve: if prob_reserve_set > 1. {
                1.
            } else {
                prob_reserve_set
            } as f32,
        }))
    } else {
        Json(None)
    }
}

async fn average_mixnode_uptime(
    identity: &str,
    current_epoch: Option<Interval>,
    storage: &State<ValidatorApiStorage>,
) -> Result<Uptime, ErrorResponse> {
    Ok(if let Some(epoch) = current_epoch {
        storage
            .get_average_mixnode_uptime_in_the_last_24hrs(identity, epoch.end_unix_timestamp())
            .await
            .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))?
    } else {
        Uptime::default()
    })
}

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
