// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::Uptime;
use mixnet_contract_common::reward_params::RewardingParams;
use mixnet_contract_common::rewarding::RewardEstimate;
use mixnet_contract_common::MixNodeBond;

pub fn compute_apy(epochs_per_hour: f64, reward: f64, pledge_amount: f64) -> f64 {
    epochs_per_hour * 24.0 * 365.0 * 100.0 * reward / pledge_amount
}

pub fn compute_reward_estimate(
    mixnode_bond: &MixNodeBond,
    uptime: Uptime,
    is_active: bool,
    interval_reward_params: RewardingParams,
    current_operator_base_cost: u64,
) -> RewardEstimate {
    todo!()
    // let node_reward_params = NodeRewardParams::new(0, u128::from(uptime.u8()), is_active);
    // let reward_params = RewardParams::new(interval_reward_params, node_reward_params);
    //
    // mixnode_bond
    //     .estimate_reward(current_operator_base_cost, &reward_params)
    //     .unwrap_or(RewardEstimate {
    //         total_node_reward: 0,
    //         operator_reward: 0,
    //         delegators_reward: 0,
    //         node_profit: 0,
    //         operator_cost: 0,
    //     })
}

pub fn compute_apy_from_reward(
    mixnode_bond: &MixNodeBond,
    reward_estimate: RewardEstimate,
    epochs_in_interval: u64,
) -> (f64, f64) {
    todo!()
    // let epochs_per_hour = epochs_in_interval as f64 / 720.0;
    // let pledge = mixnode_bond.pledge_amount().amount.u128();
    // let total_delegations = mixnode_bond.total_delegation().amount.u128();
    // let estimated_operator_apy = compute_apy(
    //     epochs_per_hour,
    //     reward_estimate.operator_reward as f64,
    //     pledge as f64,
    // );
    // let estimated_delegators_apy = compute_apy(
    //     epochs_per_hour,
    //     reward_estimate.delegators_reward as f64,
    //     total_delegations as f64,
    // );
    // (estimated_operator_apy, estimated_delegators_apy)
}
