use log::{info, trace};
use std::collections::HashMap;

use crate::ephemera::contract::MixnodeToReward;

pub(crate) struct RewardsAggregator;

impl RewardsAggregator {
    //Simple mean average
    pub(crate) fn aggregate(&self, all_rewards: Vec<Vec<MixnodeToReward>>) -> Vec<MixnodeToReward> {
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
            let sum: u8 = rewards.iter().sum();
            let avg = sum / rewards.len() as u8;
            mean_avg.push(MixnodeToReward {
                mix_id,
                performance: avg,
            });
        }
        info!("Mean average rewards: {:?}", mean_avg);

        mean_avg
    }
}
