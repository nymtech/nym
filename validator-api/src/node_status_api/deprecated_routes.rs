// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{
    ErrorResponse, MixnodeStatusReport, MixnodeUptimeHistory, Uptime,
};
use crate::node_status_api::routes::{
    _get_mixnode_reward_estimation, _get_mixnode_status, _mixnode_core_status_count,
    _mixnode_report, _mixnode_uptime_history,
};
use crate::{ValidatorApiStorage, ValidatorCache};
use crypto::asymmetric::identity;
use mixnet_contract_common::mixnode::MixNodeDetails;
use mixnet_contract_common::reward_params::RewardingParams;
use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;
use mixnet_contract_common::{IdentityKey, Interval, NodeId};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use validator_api_requests::models::{
    DeprecatedRewardEstimationResponse, InclusionProbabilityResponse, MixnodeCoreStatusResponse,
    MixnodeStatus, MixnodeStatusResponse, StakeSaturationResponse, UptimeResponse,
};

pub trait Deprecatable {
    fn deprecate(self) -> Deprecated<Self>
    where
        Self: Sized,
    {
        self.into()
    }
}

impl<T> Deprecatable for T {}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Deprecated<T> {
    deprecated: bool,
    #[serde(flatten)]
    response: T,
}

impl<T> From<T> for Deprecated<T> {
    fn from(response: T) -> Self {
        Deprecated {
            deprecated: true,
            response,
        }
    }
}

// Note: this is a very dangerous method to call as the same identity in the past might have
// referred to a completely different node id!
async fn mixnode_identity_to_current_node_id(
    storage: &ValidatorApiStorage,
    identity: &str,
) -> Result<NodeId, ErrorResponse> {
    storage
        .mix_identity_to_latest_mix_id(identity)
        .await
        .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))?
        .ok_or(ErrorResponse::new(
            "no mixnode with provided identity",
            Status::NotFound,
        ))
}
//
// enum LegacyQueryRouter {
//     Deprecated(IdentityKey),
//     Updated(NodeId),
//     Invalid(String),
// }
//
// impl From<&str> for LegacyQueryRouter {
//     fn from(raw: &str) -> Self {
//         if identity::PublicKey::from_base58_string(raw).is_ok() {
//             return LegacyQueryRouter::Deprecated(raw.into());
//         } else if let Ok(parsed_id) = raw.parse() {
//             return LegacyQueryRouter::Updated(parsed_id);
//         } else {
//             LegacyQueryRouter::Invalid(raw.into())
//         }
//     }
// }

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
    mixnode: &MixNodeDetails,
    reward_params: RewardingParams,
    as_at: i64,
) -> Result<Json<DeprecatedRewardEstimationResponse>, ErrorResponse> {
    todo!()
    // match mixnode_bond.estimate_reward(base_operator_cost, &reward_params) {
    //     Ok(reward_estimate) => {
    //         let response = DeprecatedRewardEstimationResponse {
    //             estimated_total_node_reward: reward_estimate.total_node_reward,
    //             estimated_operator_reward: reward_estimate.operator_reward,
    //             estimated_delegators_reward: reward_estimate.delegators_reward,
    //             estimated_node_profit: reward_estimate.node_profit,
    //             estimated_operator_cost: reward_estimate.operator_cost,
    //             reward_params,
    //             as_at,
    //         };
    //         Ok(Json(response))
    //     }
    //     Err(e) => Err(ErrorResponse::new(
    //         e.to_string(),
    //         Status::InternalServerError,
    //     )),
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
#[get("/mixnode/deprecated/<identity>/stake-saturation")]
pub(crate) async fn get_mixnode_stake_saturation_by_identity(
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
#[get("/mixnode/deprecated/<identity>/inclusion-probability")]
pub(crate) async fn get_mixnode_inclusion_probability_by_identity(
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
#[get("/mixnode/deprecated/<identity>/report")]
pub(crate) async fn mixnode_report_by_identity(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<Deprecated<MixnodeStatusReport>>, ErrorResponse> {
    let mix_id = mixnode_identity_to_current_node_id(storage, identity).await?;
    Ok(Json(_mixnode_report(storage, mix_id).await?.deprecate()))
}

#[openapi(tag = "status")]
#[get("/mixnode/deprecated/<identity>/history")]
pub(crate) async fn mixnode_uptime_history_by_identity(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<Deprecated<MixnodeUptimeHistory>>, ErrorResponse> {
    let mix_id = mixnode_identity_to_current_node_id(storage, identity).await?;
    Ok(Json(
        _mixnode_uptime_history(storage, mix_id).await?.deprecate(),
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/deprecated/<identity>/core-status-count?<since>")]
pub(crate) async fn mixnode_core_status_count_by_identity(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
    since: Option<i64>,
) -> Result<Json<Deprecated<MixnodeCoreStatusResponse>>, ErrorResponse> {
    let mix_id = mixnode_identity_to_current_node_id(storage, identity).await?;
    Ok(Json(
        _mixnode_core_status_count(storage, mix_id, since)
            .await?
            .deprecate(),
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/deprecated/<identity>/status")]
pub(crate) async fn get_mixnode_status_by_identity(
    storage: &State<ValidatorApiStorage>,
    cache: &State<ValidatorCache>,
    identity: &str,
) -> Json<Deprecated<MixnodeStatusResponse>> {
    match mixnode_identity_to_current_node_id(storage, identity).await {
        Ok(mix_id) => Json(_get_mixnode_status(cache, mix_id).await.deprecate()),
        Err(_) => Json(
            MixnodeStatusResponse {
                status: MixnodeStatus::NotFound,
            }
            .deprecate(),
        ),
    }
}

#[openapi(tag = "status")]
#[get("/mixnode/deprecated/<identity>/reward-estimation")]
pub(crate) async fn get_mixnode_reward_estimation_by_identity(
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<DeprecatedRewardEstimationResponse>, ErrorResponse> {
    let mix_id = mixnode_identity_to_current_node_id(storage, identity).await?;
    let new_estimation = _get_mixnode_reward_estimation(cache, mix_id).await?;

    Ok(Json(DeprecatedRewardEstimationResponse {
        estimated_total_node_reward: truncate_reward_amount(
            new_estimation.estimation.total_node_reward,
        )
        .u128()
        .try_into()
        .unwrap_or_default(),
        estimated_operator_reward: truncate_reward_amount(new_estimation.estimation.operator)
            .u128()
            .try_into()
            .unwrap_or_default(),
        estimated_delegators_reward: truncate_reward_amount(new_estimation.estimation.delegates)
            .u128()
            .try_into()
            .unwrap_or_default(),
        estimated_node_profit: if new_estimation.estimation.operator
            < new_estimation.estimation.operating_cost
        {
            0
        } else {
            truncate_reward_amount(
                new_estimation.estimation.operator - new_estimation.estimation.operating_cost,
            )
            .u128()
            .try_into()
            .unwrap_or_default()
        },
        estimated_operator_cost: truncate_reward_amount(new_estimation.estimation.operating_cost)
            .u128()
            .try_into()
            .unwrap_or_default(),
        reward_params: new_estimation.reward_params,
        as_at: new_estimation.as_at,
    }))
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct DeprecatedComputeRewardEstParam {
    uptime: Option<u8>,
    is_active: Option<bool>,
    pledge_amount: Option<u64>,
    total_delegation: Option<u64>,
}

#[openapi(tag = "status")]
#[post(
    "/mixnode/deprecated/<identity>/compute-reward-estimation",
    data = "<user_reward_param>"
)]
pub(crate) async fn compute_mixnode_reward_estimation_by_identity(
    user_reward_param: Json<ComputeRewardEstParam>,
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<DeprecatedRewardEstimationResponse>, ErrorResponse> {
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
#[get("/mixnode/deprecated/<identity>/avg_uptime")]
pub(crate) async fn get_mixnode_avg_uptime_by_identity(
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
    identity: &str,
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
#[get("/mixnodes/deprecated/avg_uptime")]
pub(crate) async fn get_mixnode_avg_uptimes_by_identity(
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
) -> Result<Json<Vec<Deprecated<UptimeResponse>>>, ErrorResponse> {
    todo!()
    // let mixnodes = cache.mixnodes().await;
    // let current_epoch = cache.current_epoch().await.into_inner();
    //
    // let mut response = Vec::new();
    // for mixnode in mixnodes {
    //     let uptime = average_mixnode_uptime(mixnode.identity(), current_epoch, storage).await?;
    //
    //     response.push(
    //         UptimeResponse {
    //             identity: mixnode.identity().to_string(),
    //             avg_uptime: uptime.u8(),
    //         }
    //         .deprecate(),
    //     )
    // }
    //
    // Ok(Json(response))
}
