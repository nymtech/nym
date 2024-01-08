// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_std::Decimal;
use nym_mixnet_contract_common::mixnode::MixNodeDetails;
use nym_mixnet_contract_common::reward_params::{NodeRewardParams, Performance, RewardingParams};
use nym_mixnet_contract_common::rewarding::RewardEstimate;
use nym_mixnet_contract_common::{Interval, RewardedSetNodeStatus};

fn compute_apy(epochs_in_year: Decimal, reward: Decimal, pledge_amount: Decimal) -> Decimal {
    if pledge_amount.is_zero() {
        return Decimal::zero();
    }
    let hundred = Decimal::from_ratio(100u32, 1u32);

    epochs_in_year * hundred * reward / pledge_amount
}

pub fn compute_reward_estimate(
    mixnode: &MixNodeDetails,
    performance: Performance,
    rewarded_set_status: Option<RewardedSetNodeStatus>,
    rewarding_params: RewardingParams,
    interval: Interval,
) -> RewardEstimate {
    if mixnode.is_unbonding() {
        return Default::default();
    }

    if performance.is_zero() {
        return Default::default();
    }

    let node_status = match rewarded_set_status {
        Some(status) => status,
        // if node is not in the rewarded set, it's not going to get anything
        None => return Default::default(),
    };

    let node_reward_params = NodeRewardParams::new(performance, node_status.is_active());
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
    mixnode: &MixNodeDetails,
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
