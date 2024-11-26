// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::helpers::RewardedSetStatus;
use cosmwasm_std::Decimal;
use nym_api_requests::legacy::LegacyMixNodeDetailsWithLayer;
use nym_mixnet_contract_common::reward_params::{
    NodeRewardingParameters, Performance, RewardingParams,
};
use nym_mixnet_contract_common::rewarding::RewardEstimate;
use nym_mixnet_contract_common::Interval;

fn compute_apy(epochs_in_year: Decimal, reward: Decimal, pledge_amount: Decimal) -> Decimal {
    if pledge_amount.is_zero() {
        return Decimal::zero();
    }
    let hundred = Decimal::from_ratio(100u32, 1u32);

    epochs_in_year * hundred * reward / pledge_amount
}

pub fn compute_reward_estimate(
    mixnode: &LegacyMixNodeDetailsWithLayer,
    performance: Performance,
    rewarded_set_status: RewardedSetStatus,
    rewarding_params: RewardingParams,
    interval: Interval,
) -> RewardEstimate {
    if mixnode.is_unbonding() {
        return Default::default();
    }

    if performance.is_zero() {
        return Default::default();
    }

    let is_active = match rewarded_set_status {
        RewardedSetStatus::Active => true,
        RewardedSetStatus::Standby => false,
        RewardedSetStatus::Inactive => return Default::default(),
    };

    let work_factor = if is_active {
        rewarding_params.active_node_work()
    } else {
        rewarding_params.standby_node_work()
    };

    let node_reward_params = NodeRewardingParameters {
        performance,
        work_factor,
    };
    let node_reward = mixnode
        .rewarding_details
        .node_reward(&rewarding_params, node_reward_params);

    let node_cost = mixnode
        .rewarding_details
        .cost_params
        .epoch_operating_cost(interval.epochs_in_interval())
        * performance;

    let reward_distribution = mixnode.rewarding_details.determine_reward_split(
        node_reward,
        performance,
        interval.epochs_in_interval(),
    );

    RewardEstimate {
        total_node_reward: node_reward,
        operator: reward_distribution.operator,
        delegates: reward_distribution.delegates,
        operating_cost: node_cost,
    }
}

pub fn compute_apy_from_reward(
    mixnode: &LegacyMixNodeDetailsWithLayer,
    reward_estimate: RewardEstimate,
    interval: Interval,
) -> (Decimal, Decimal) {
    let epochs_in_year = Decimal::from_ratio(interval.epoch_length_secs(), 3600u64 * 24 * 365);

    let operator = mixnode.rewarding_details.operator;
    let total_delegations = mixnode.rewarding_details.delegates;
    let estimated_operator_apy = compute_apy(epochs_in_year, reward_estimate.operator, operator);
    let estimated_delegators_apy =
        compute_apy(epochs_in_year, reward_estimate.delegates, total_delegations);
    (estimated_operator_apy, estimated_delegators_apy)
}
