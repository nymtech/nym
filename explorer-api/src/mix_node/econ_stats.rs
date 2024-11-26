// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::ThreadsafeValidatorClient;
use crate::helpers::best_effort_small_dec_to_f64;
use crate::mix_node::models::EconomicDynamicsStats;
use nym_contracts_common::truncate_decimal;
use nym_mixnet_contract_common::NodeId;
use nym_validator_client::client::NymApiClientExt;

// use deprecated method as hopefully this whole API will be sunset soon-enough...
// and we're only getting info for legacy node so the relevant data should still exist
#[allow(deprecated)]
pub(crate) async fn retrieve_mixnode_econ_stats(
    client: &ThreadsafeValidatorClient,
    mix_id: NodeId,
) -> Option<EconomicDynamicsStats> {
    let stake_saturation = client
        .0
        .nym_api
        .get_mixnode_stake_saturation(mix_id)
        .await
        .ok()?;

    let inclusion_probability = client
        .0
        .nym_api
        .get_mixnode_inclusion_probability(mix_id)
        .await
        .ok()?;

    let reward_estimation = client
        .0
        .nym_api
        .get_mixnode_reward_estimation(mix_id)
        .await
        .ok()?;

    let uptime_response = client.0.nym_api.get_mixnode_avg_uptime(mix_id).await.ok()?;

    Some(EconomicDynamicsStats {
        stake_saturation: best_effort_small_dec_to_f64(stake_saturation.saturation) as f32,
        uncapped_saturation: best_effort_small_dec_to_f64(stake_saturation.uncapped_saturation)
            as f32,
        active_set_inclusion_probability: inclusion_probability.in_active,
        reserve_set_inclusion_probability: inclusion_probability.in_reserve,
        // drop precision for compatibility sake
        estimated_total_node_reward: truncate_decimal(
            reward_estimation.estimation.total_node_reward,
        )
        .u128() as u64,
        estimated_operator_reward: truncate_decimal(reward_estimation.estimation.operator).u128()
            as u64,
        estimated_delegators_reward: truncate_decimal(reward_estimation.estimation.delegates).u128()
            as u64,
        current_interval_uptime: uptime_response.avg_uptime,
    })
}
