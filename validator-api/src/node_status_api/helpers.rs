// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract_cache::reward_estimate::compute_reward_estimate;
use crate::node_status_api::models::{ErrorResponse, MixnodeStatusReport, MixnodeUptimeHistory};
use crate::storage::ValidatorApiStorage;
use crate::ValidatorCache;
use cosmwasm_std::Decimal;
use mixnet_contract_common::reward_params::Performance;
use mixnet_contract_common::{Interval, NodeId, RewardedSetNodeStatus};
use rocket::http::Status;
use rocket::State;
use validator_api_requests::models::{
    ComputeRewardEstParam, InclusionProbabilityResponse, MixnodeCoreStatusResponse,
    MixnodeStatusResponse, RewardEstimationResponse, StakeSaturationResponse, UptimeResponse,
};

pub(crate) async fn _mixnode_report(
    storage: &ValidatorApiStorage,
    mix_id: NodeId,
) -> Result<MixnodeStatusReport, ErrorResponse> {
    storage
        .construct_mixnode_report(mix_id)
        .await
        .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))
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

pub(crate) async fn _get_mixnode_status(
    cache: &ValidatorCache,
    mix_id: NodeId,
) -> MixnodeStatusResponse {
    MixnodeStatusResponse {
        status: cache.mixnode_status(mix_id).await,
    }
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

async fn average_mixnode_performance(
    mix_id: NodeId,
    current_interval: Interval,
    storage: &ValidatorApiStorage,
) -> Result<Performance, ErrorResponse> {
    storage
        .get_average_mixnode_uptime_in_the_last_24hrs(
            mix_id,
            current_interval.current_epoch_end_unix_timestamp(),
        )
        .await
        .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))
        .map(Into::into)
}

pub(crate) async fn _compute_mixnode_reward_estimation(
    user_reward_param: ComputeRewardEstParam,
    cache: &ValidatorCache,
    mix_id: NodeId,
) -> Result<RewardEstimationResponse, ErrorResponse> {
    let (mixnode, actual_status) = cache.mixnode_details(mix_id).await;
    if let Some(mut mixnode) = mixnode {
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

        // For these parameters we either use the provided ones, or fall back to the system ones
        let performance = user_reward_param.performance.unwrap_or(mixnode.performance);

        let status = match user_reward_param.active_in_rewarded_set {
            Some(true) => Some(RewardedSetNodeStatus::Active),
            Some(false) => Some(RewardedSetNodeStatus::Standby),
            None => actual_status.into(),
        };

        if let Some(pledge_amount) = user_reward_param.pledge_amount {
            mixnode.mixnode_details.rewarding_details.operator =
                Decimal::from_ratio(pledge_amount, 1u64);
        }
        if let Some(total_delegation) = user_reward_param.total_delegation {
            mixnode.mixnode_details.rewarding_details.delegates =
                Decimal::from_ratio(total_delegation, 1u64);
        }

        let reward_estimation = compute_reward_estimate(
            &mixnode.mixnode_details,
            performance,
            status,
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

pub(crate) async fn _get_mixnode_stake_saturation(
    cache: &ValidatorCache,
    mix_id: NodeId,
) -> Result<StakeSaturationResponse, ErrorResponse> {
    let (mixnode, _) = cache.mixnode_details(mix_id).await;
    if let Some(mixnode) = mixnode {
        // Recompute the stake saturation just so that we can confidently state that the `as_at`
        // field is consistent and correct. Luckily this is very cheap.
        let reward_params = cache.interval_reward_params().await;
        let as_at = reward_params.timestamp();
        let rewarding_params = reward_params
            .into_inner()
            .ok_or_else(|| ErrorResponse::new("server error", Status::InternalServerError))?;

        Ok(StakeSaturationResponse {
            saturation: mixnode
                .mixnode_details
                .rewarding_details
                .bond_saturation(&rewarding_params),
            uncapped_saturation: mixnode
                .mixnode_details
                .rewarding_details
                .uncapped_bond_saturation(&rewarding_params),
            as_at,
        })
    } else {
        Err(ErrorResponse::new(
            "mixnode bond not found",
            Status::NotFound,
        ))
    }
}

pub(crate) async fn _get_mixnode_inclusion_probability(
    cache: &ValidatorCache,
    mix_id: NodeId,
) -> Result<InclusionProbabilityResponse, ErrorResponse> {
    let (mixnode, _) = cache.mixnode_details(mix_id).await;
    if let Some(mixnode) = mixnode {
        let all_nodes = cache.mixnodes().await;
        let total_bonded_tokens: Decimal = all_nodes.iter().map(|mix| mix.total_stake()).sum();

        let reward_params = cache.interval_reward_params().await;
        let rewarding_params = reward_params
            .into_inner()
            .ok_or_else(|| ErrorResponse::new("server error", Status::InternalServerError))?;

        let rewarded_set_size = rewarding_params.dec_rewarded_set_size();
        let active_set_size = rewarding_params.dec_active_set_size();

        let prob_one_draw = mixnode.mixnode_details.total_stake() / total_bonded_tokens;
        // Chance to be selected in any draw for active set
        let prob_active_set = if all_nodes.len() <= rewarding_params.active_set_size as usize {
            Decimal::one()
        } else {
            // @JS: NOTE: this is way overestimated because for any future draws you're drawing from a smaller set
            active_set_size * prob_one_draw
        };
        // This is likely slightly too high, as we're not correcting form them not being selected in active, should be chance to be selected, minus the chance for being not selected in reserve
        let prob_reserve_set = if all_nodes.len() <= rewarding_params.rewarded_set_size as usize {
            Decimal::one()
        } else {
            (rewarded_set_size - active_set_size) * prob_one_draw
        };

        Ok(InclusionProbabilityResponse {
            in_active: prob_active_set.into(),
            in_reserve: prob_reserve_set.into(),
        })
    } else {
        Err(ErrorResponse::new(
            "mixnode bond not found",
            Status::NotFound,
        ))
    }
}

pub(crate) async fn _get_mixnode_avg_uptime(
    cache: &ValidatorCache,
    storage: &ValidatorApiStorage,
    mix_id: NodeId,
) -> Result<UptimeResponse, ErrorResponse> {
    let current_interval = cache
        .current_interval()
        .await
        .into_inner()
        .ok_or_else(|| ErrorResponse::new("server error", Status::InternalServerError))?;

    let performance = average_mixnode_performance(mix_id, current_interval, storage).await?;

    Ok(UptimeResponse {
        mix_id,
        avg_uptime: performance.round_to_integer(),
    })
}
