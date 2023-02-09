// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::epoch_operations::error::RewardingError;
use crate::support::storage::models::RewardingReport;
use crate::RewardedSetUpdater;
use mixnet_contract_common::reward_params::Performance;
use mixnet_contract_common::{ExecuteMsg, Interval, MixId};

#[derive(Debug, Clone, Copy)]
pub(crate) struct MixnodeToReward {
    pub(crate) mix_id: MixId,

    pub(crate) performance: Performance,
}

impl From<MixnodeToReward> for ExecuteMsg {
    fn from(mix_reward: MixnodeToReward) -> Self {
        ExecuteMsg::RewardMixnode {
            mix_id: mix_reward.mix_id,
            performance: mix_reward.performance,
        }
    }
}

impl RewardedSetUpdater {
    pub(super) async fn reward_current_rewarded_set(
        &self,
        current_interval: Interval,
    ) -> Result<(), RewardingError> {
        let to_reward = self.nodes_to_reward(current_interval).await;

        if let Some(existing_report) = self
            .storage
            .get_rewarding_report(current_interval.current_epoch_absolute_id())
            .await?
        {
            warn!("We have already rewarded mixnodes for this rewarding epoch ({}). {} nodes should have gotten rewards", existing_report.absolute_epoch_id, existing_report.eligible_mixnodes);
            return Ok(());
        }

        if to_reward.is_empty() {
            info!("There are no nodes to reward in this epoch");
        } else if let Err(err) = self.nyxd_client.send_rewarding_messages(&to_reward).await {
            error!(
                "failed to perform mixnode rewarding for epoch {}! Error encountered: {err}",
                current_interval.current_epoch_absolute_id(),
            );
            return Err(err.into());
        }

        log::info!("rewarded {} mixnodes...", to_reward.len());

        let rewarding_report = RewardingReport {
            absolute_epoch_id: current_interval.current_epoch_absolute_id(),
            eligible_mixnodes: to_reward.len() as u32,
        };

        self.storage
            .insert_rewarding_report(rewarding_report)
            .await?;

        Ok(())
    }

    async fn nodes_to_reward(&self, interval: Interval) -> Vec<MixnodeToReward> {
        // try to get current up to date view of the network bypassing the cache
        // in case the epochs were significantly shortened for the purposes of testing
        let rewarded_set: Vec<MixId> = match self.nyxd_client.get_rewarded_set_mixnodes().await {
            Ok(nodes) => nodes.into_iter().map(|(id, _)| id).collect::<Vec<_>>(),
            Err(err) => {
                warn!("failed to obtain the current rewarded set - {err}. falling back to the cached version");
                self.nym_contract_cache
                    .rewarded_set()
                    .await
                    .into_inner()
                    .into_iter()
                    .map(|node| node.mix_id())
                    .collect::<Vec<_>>()
            }
        };

        let mut eligible_nodes = Vec::with_capacity(rewarded_set.len());
        for mix_id in rewarded_set {
            let uptime = self
                .storage
                .get_average_mixnode_uptime_in_the_last_24hrs(
                    mix_id,
                    interval.current_epoch_end_unix_timestamp(),
                )
                .await
                .unwrap_or_default();
            eligible_nodes.push(MixnodeToReward {
                mix_id,
                performance: uptime.into(),
            })
        }

        eligible_nodes
    }
}
