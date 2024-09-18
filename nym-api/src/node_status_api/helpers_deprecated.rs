// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::RocketErrorResponse;
use crate::storage::NymApiStorage;
use crate::support::caching::Cache;
use crate::{NodeStatusCache, NymContractCache};
use cosmwasm_std::Decimal;
use nym_api_requests::models::{
    AllInclusionProbabilitiesResponse, ComputeRewardEstParam, GatewayBondAnnotated,
    GatewayCoreStatusResponse, GatewayStatusReportResponse, GatewayUptimeHistoryResponse,
    GatewayUptimeResponse, InclusionProbabilityResponse, MixNodeBondAnnotated,
    MixnodeCoreStatusResponse, MixnodeStatusReportResponse, MixnodeStatusResponse,
    MixnodeUptimeHistoryResponse, RewardEstimationResponse, StakeSaturationResponse,
    UptimeResponse,
};
use nym_mixnet_contract_common::{MixId, RewardedSetNodeStatus};
use rocket::http::Status;
use rocket::State;

use super::reward_estimate::compute_reward_estimate;

async fn get_gateway_bond_annotated(
    cache: &NodeStatusCache,
    identity: &str,
) -> Result<GatewayBondAnnotated, RocketErrorResponse> {
    cache
        .gateway_annotated(identity)
        .await
        .ok_or(RocketErrorResponse::new(
            "gateway bond not found",
            Status::NotFound,
        ))
}

async fn get_mixnode_bond_annotated(
    cache: &NodeStatusCache,
    mix_id: MixId,
) -> Result<MixNodeBondAnnotated, RocketErrorResponse> {
    cache
        .mixnode_annotated(mix_id)
        .await
        .ok_or(RocketErrorResponse::new(
            "mixnode bond not found",
            Status::NotFound,
        ))
}

pub(crate) async fn _gateway_report(
    cache: &NodeStatusCache,
    identity: &str,
) -> Result<GatewayStatusReportResponse, RocketErrorResponse> {
    let gateway = get_gateway_bond_annotated(cache, identity).await?;

    Ok(GatewayStatusReportResponse {
        identity: gateway.identity().to_owned(),
        owner: gateway.owner().to_string(),
        most_recent: gateway.node_performance.most_recent.round_to_integer(),
        last_hour: gateway.node_performance.last_hour.round_to_integer(),
        last_day: gateway.node_performance.last_24h.round_to_integer(),
    })
}

pub(crate) async fn _gateway_uptime_history(
    storage: &NymApiStorage,
    identity: &str,
) -> Result<GatewayUptimeHistoryResponse, RocketErrorResponse> {
    storage
        .get_gateway_uptime_history(identity)
        .await
        .map(GatewayUptimeHistoryResponse::from)
        .map_err(|err| RocketErrorResponse::new(err.to_string(), Status::NotFound))
}

pub(crate) async fn _gateway_core_status_count(
    storage: &State<NymApiStorage>,
    identity: &str,
    since: Option<i64>,
) -> Result<GatewayCoreStatusResponse, RocketErrorResponse> {
    let count = storage
        .get_core_gateway_status_count(identity, since)
        .await
        .map_err(|err| RocketErrorResponse::new(err.to_string(), Status::NotFound))?;

    Ok(GatewayCoreStatusResponse {
        identity: identity.to_string(),
        count,
    })
}

pub(crate) async fn _mixnode_report(
    cache: &NodeStatusCache,
    mix_id: MixId,
) -> Result<MixnodeStatusReportResponse, RocketErrorResponse> {
    let mixnode = get_mixnode_bond_annotated(cache, mix_id).await?;

    Ok(MixnodeStatusReportResponse {
        mix_id,
        identity: mixnode.identity_key().to_owned(),
        owner: mixnode.owner().to_string(),
        most_recent: mixnode.node_performance.most_recent.round_to_integer(),
        last_hour: mixnode.node_performance.last_hour.round_to_integer(),
        last_day: mixnode.node_performance.last_24h.round_to_integer(),
    })
}

pub(crate) async fn _mixnode_uptime_history(
    storage: &NymApiStorage,
    mix_id: MixId,
) -> Result<MixnodeUptimeHistoryResponse, RocketErrorResponse> {
    storage
        .get_mixnode_uptime_history(mix_id)
        .await
        .map(MixnodeUptimeHistoryResponse::from)
        .map_err(|err| RocketErrorResponse::new(err.to_string(), Status::NotFound))
}

pub(crate) async fn _mixnode_core_status_count(
    storage: &State<NymApiStorage>,
    mix_id: MixId,
    since: Option<i64>,
) -> Result<MixnodeCoreStatusResponse, RocketErrorResponse> {
    let count = storage
        .get_core_mixnode_status_count(mix_id, since)
        .await
        .map_err(|err| RocketErrorResponse::new(err.to_string(), Status::NotFound))?;

    Ok(MixnodeCoreStatusResponse { mix_id, count })
}

pub(crate) async fn _get_mixnode_status(
    cache: &NymContractCache,
    mix_id: MixId,
) -> MixnodeStatusResponse {
    MixnodeStatusResponse {
        status: cache.mixnode_status(mix_id).await,
    }
}

pub(crate) async fn _get_mixnode_reward_estimation(
    cache: &State<NodeStatusCache>,
    validator_cache: &State<NymContractCache>,
    mix_id: MixId,
) -> Result<RewardEstimationResponse, RocketErrorResponse> {
    let (mixnode, status) = cache.mixnode_details(mix_id).await;
    if let Some(mixnode) = mixnode {
        let reward_params = validator_cache.interval_reward_params().await;
        let as_at = reward_params.timestamp();
        let reward_params = reward_params
            .into_inner()
            .ok_or_else(|| RocketErrorResponse::new("server error", Status::InternalServerError))?;
        let current_interval = validator_cache
            .current_interval()
            .await
            .into_inner()
            .ok_or_else(|| RocketErrorResponse::new("server error", Status::InternalServerError))?;

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
            as_at: as_at.unix_timestamp(),
        })
    } else {
        Err(RocketErrorResponse::new(
            "mixnode bond not found",
            Status::NotFound,
        ))
    }
}

pub(crate) async fn _compute_mixnode_reward_estimation(
    user_reward_param: ComputeRewardEstParam,
    cache: &NodeStatusCache,
    validator_cache: &NymContractCache,
    mix_id: MixId,
) -> Result<RewardEstimationResponse, RocketErrorResponse> {
    let (mixnode, actual_status) = cache.mixnode_details(mix_id).await;
    if let Some(mut mixnode) = mixnode {
        let reward_params = validator_cache.interval_reward_params().await;
        let as_at = reward_params.timestamp();
        let reward_params = reward_params
            .into_inner()
            .ok_or_else(|| RocketErrorResponse::new("server error", Status::InternalServerError))?;
        let current_interval = validator_cache
            .current_interval()
            .await
            .into_inner()
            .ok_or_else(|| RocketErrorResponse::new("server error", Status::InternalServerError))?;

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

        if let Some(profit_margin_percent) = user_reward_param.profit_margin_percent {
            mixnode
                .mixnode_details
                .rewarding_details
                .cost_params
                .profit_margin_percent = profit_margin_percent;
        }

        if let Some(interval_operating_cost) = user_reward_param.interval_operating_cost {
            mixnode
                .mixnode_details
                .rewarding_details
                .cost_params
                .interval_operating_cost = interval_operating_cost;
        }

        if mixnode.mixnode_details.rewarding_details.operator
            + mixnode.mixnode_details.rewarding_details.delegates
            > reward_params.interval.staking_supply
        {
            return Err(RocketErrorResponse::new(
                "Pledge plus delegation too large",
                Status::UnprocessableEntity,
            ));
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
            as_at: as_at.unix_timestamp(),
        })
    } else {
        Err(RocketErrorResponse::new(
            "mixnode bond not found",
            Status::NotFound,
        ))
    }
}

pub(crate) async fn _get_mixnode_stake_saturation(
    cache: &NodeStatusCache,
    validator_cache: &NymContractCache,
    mix_id: MixId,
) -> Result<StakeSaturationResponse, RocketErrorResponse> {
    let (mixnode, _) = cache.mixnode_details(mix_id).await;
    if let Some(mixnode) = mixnode {
        // Recompute the stake saturation just so that we can confidently state that the `as_at`
        // field is consistent and correct. Luckily this is very cheap.
        let reward_params = validator_cache.interval_reward_params().await;
        let as_at = reward_params.timestamp();
        let rewarding_params = reward_params
            .into_inner()
            .ok_or_else(|| RocketErrorResponse::new("server error", Status::InternalServerError))?;

        Ok(StakeSaturationResponse {
            saturation: mixnode
                .mixnode_details
                .rewarding_details
                .bond_saturation(&rewarding_params),
            uncapped_saturation: mixnode
                .mixnode_details
                .rewarding_details
                .uncapped_bond_saturation(&rewarding_params),
            as_at: as_at.unix_timestamp(),
        })
    } else {
        Err(RocketErrorResponse::new(
            "mixnode bond not found",
            Status::NotFound,
        ))
    }
}

pub(crate) async fn _get_mixnode_inclusion_probability(
    cache: &NodeStatusCache,
    mix_id: MixId,
) -> Result<InclusionProbabilityResponse, RocketErrorResponse> {
    cache
        .inclusion_probabilities()
        .await
        .map(Cache::into_inner)
        .and_then(|p| p.node(mix_id).cloned())
        .map(|p| InclusionProbabilityResponse {
            in_active: p.in_active.into(),
            in_reserve: p.in_reserve.into(),
        })
        .ok_or_else(|| RocketErrorResponse::new("mixnode bond not found", Status::NotFound))
}

pub(crate) async fn _get_mixnode_avg_uptime(
    cache: &NodeStatusCache,
    mix_id: MixId,
) -> Result<UptimeResponse, RocketErrorResponse> {
    let mixnode = get_mixnode_bond_annotated(cache, mix_id).await?;

    Ok(UptimeResponse {
        mix_id,
        avg_uptime: mixnode.node_performance.last_24h.round_to_integer(),
        node_performance: mixnode.node_performance,
    })
}

pub(crate) async fn _get_gateway_avg_uptime(
    cache: &NodeStatusCache,
    identity: &str,
) -> Result<GatewayUptimeResponse, RocketErrorResponse> {
    let gateway = get_gateway_bond_annotated(cache, identity).await?;

    Ok(GatewayUptimeResponse {
        identity: identity.to_string(),
        avg_uptime: gateway.node_performance.last_24h.round_to_integer(),
        node_performance: gateway.node_performance,
    })
}

pub(crate) async fn _get_mixnode_inclusion_probabilities(
    cache: &NodeStatusCache,
) -> Result<AllInclusionProbabilitiesResponse, RocketErrorResponse> {
    if let Some(prob) = cache.inclusion_probabilities().await {
        let as_at = prob.timestamp();
        let prob = prob.into_inner();
        Ok(AllInclusionProbabilitiesResponse {
            inclusion_probabilities: prob.inclusion_probabilities,
            samples: prob.samples,
            elapsed: prob.elapsed,
            delta_max: prob.delta_max,
            delta_l2: prob.delta_l2,
            as_at: as_at.unix_timestamp(),
        })
    } else {
        Err(RocketErrorResponse::new(
            "No data available",
            Status::ServiceUnavailable,
        ))
    }
}

pub(crate) async fn _get_mixnodes_detailed(cache: &NodeStatusCache) -> Vec<MixNodeBondAnnotated> {
    cache
        .mixnodes_annotated_filtered()
        .await
        .unwrap_or_default()
}

pub(crate) async fn _get_mixnodes_detailed_unfiltered(
    cache: &NodeStatusCache,
) -> Vec<MixNodeBondAnnotated> {
    cache.mixnodes_annotated_full().await.unwrap_or_default()
}

pub(crate) async fn _get_rewarded_set_detailed(
    cache: &NodeStatusCache,
) -> Vec<MixNodeBondAnnotated> {
    cache
        .rewarded_set_annotated()
        .await
        .unwrap_or_default()
        .into_inner()
}

pub(crate) async fn _get_active_set_detailed(cache: &NodeStatusCache) -> Vec<MixNodeBondAnnotated> {
    cache
        .active_set_annotated()
        .await
        .unwrap_or_default()
        .into_inner()
}

pub(crate) async fn _get_gateways_detailed(cache: &NodeStatusCache) -> Vec<GatewayBondAnnotated> {
    cache
        .gateways_annotated_filtered()
        .await
        .unwrap_or_default()
}

pub(crate) async fn _get_gateways_detailed_unfiltered(
    cache: &NodeStatusCache,
) -> Vec<GatewayBondAnnotated> {
    cache.gateways_annotated_full().await.unwrap_or_default()
}
