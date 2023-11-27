// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_std::Decimal;
use log::{info, trace};
use nym_mixnet_contract_common::reward_params::Performance;
use std::collections::HashMap;

use crate::epoch_operations::MixnodeWithPerformance;

pub(crate) struct RewardsAggregator;

impl RewardsAggregator {
    //Simple mean average
    pub(crate) fn aggregate(
        &self,
        all_rewards: Vec<Vec<MixnodeWithPerformance>>,
    ) -> anyhow::Result<Vec<MixnodeWithPerformance>> {
        let mut mix_rewards = HashMap::new();
        for api_rewards in all_rewards {
            for mixnode in api_rewards {
                mix_rewards
                    .entry(mixnode.mix_id)
                    .or_insert(vec![])
                    .push(mixnode.performance);
            }
        }
        trace!("Mix rewards by node: {:?}", mix_rewards);

        trace!("Calculating mean average for each node");
        let mut mean_avg = vec![];
        for (mix_id, rewards) in mix_rewards {
            let sum: Decimal = rewards.iter().map(|r| r.value()).sum();
            let avg = sum / Decimal::from_ratio(rewards.len() as u128, 1u128);
            let performance = Performance::new(avg)?;
            mean_avg.push(MixnodeWithPerformance {
                mix_id,
                performance,
            });
        }
        info!("Mean average rewards: {:?}", mean_avg);

        Ok(mean_avg)
    }
}
