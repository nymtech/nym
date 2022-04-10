// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::ThreadsafeValidatorClient;
use crate::mix_node::models::EconomicDynamicsStats;

pub(crate) async fn retrieve_mixnode_econ_stats(
    client: &ThreadsafeValidatorClient,
    identity: &str,
) -> Option<EconomicDynamicsStats> {
    let stake_saturation = client
        .0
        .validator_api
        .get_mixnode_stake_saturation(identity)
        .await
        .ok()?;

    let inclusion_probability = client
        .0
        .validator_api
        .get_mixnode_inclusion_probability(identity)
        .await
        .ok()?;

    let reward_estimation = client
        .0
        .validator_api
        .get_mixnode_reward_estimation(identity)
        .await
        .ok()?;

    Some(EconomicDynamicsStats {
        stake_saturation: stake_saturation.saturation,
        active_set_inclusion_probability: inclusion_probability.in_active,
        reserve_set_inclusion_probability: inclusion_probability.in_reserve,
        estimated_total_node_reward: reward_estimation.estimated_total_node_reward,
        estimated_operator_reward: reward_estimation.estimated_operator_reward,
        estimated_delegators_reward: reward_estimation.estimated_delegators_reward,
        current_interval_uptime: reward_estimation.reward_params.node.uptime() as u8,
    })
}
