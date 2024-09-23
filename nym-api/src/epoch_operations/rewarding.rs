// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::epoch_operations::error::RewardingError;
use crate::epoch_operations::helpers::RewardedNodeWithParams;
use crate::EpochAdvancer;
use nym_mixnet_contract_common::{EpochState, Interval};
use tracing::{error, info, warn};

impl EpochAdvancer {
    pub(super) async fn reward_current_rewarded_set(
        &self,
        to_reward: Vec<RewardedNodeWithParams>,
        current_interval: Interval,
    ) -> Result<(), RewardingError> {
        let epoch_status = self.nyxd_client.get_current_epoch_status().await?;
        match epoch_status.state {
            EpochState::InProgress => {
                // hard error, this shouldn't have happened!
                error!("tried to perform node rewarding while the epoch is still in progress!");
                Err(RewardingError::InvalidEpochState {
                    current_state: EpochState::InProgress,
                    operation: "mix rewarding".to_string(),
                })
            }
            EpochState::ReconcilingEvents | EpochState::RoleAssignment { .. } => {
                warn!("we seem to have crashed mid epoch operations... no need to reward nodes as we've already done that! (or this could be a false positive if there were no nodes to reward - to fix this warning later)");
                Ok(())
            }
            EpochState::Rewarding { last_rewarded, .. } => {
                info!("Rewarding the current rewarded set...");

                // with how the nym-api is currently coded, this should never happen as we're always
                // rewarding ALL nodes at once, but who knows what we might decide to do in the future...
                if last_rewarded != 0 {
                    return Err(RewardingError::MidNodeRewarding { last_rewarded });
                }

                if let Err(err) = self
                    ._reward_current_rewarded_set(to_reward, current_interval)
                    .await
                {
                    error!("FAILED to reward rewarded set: {err}");
                    Err(err)
                } else {
                    info!("Rewarded current rewarded set... SUCCESS");
                    Ok(())
                }
            }
        }
    }

    async fn _reward_current_rewarded_set(
        &self,
        to_reward: Vec<RewardedNodeWithParams>,
        current_interval: Interval,
    ) -> Result<(), RewardingError> {
        if to_reward.is_empty() {
            error!("There are no nodes to reward in this epoch - we shouldn't have been in the 'Rewarding' state!");
        } else if let Err(err) = self.nyxd_client.send_rewarding_messages(&to_reward).await {
            error!(
                "failed to perform node rewarding for epoch {}! Error encountered: {err}",
                current_interval.current_epoch_absolute_id(),
            );
            return Err(err.into());
        }

        info!("rewarded {} nodes...", to_reward.len());

        Ok(())
    }

    pub(crate) async fn nodes_to_reward(
        &self,
        interval: Interval,
    ) -> Result<Vec<RewardedNodeWithParams>, RewardingError> {
        // try to get current up-to-date view of the network bypassing the cache
        // in case the epochs were significantly shortened for the purposes of testing
        let rewarded_set = match self.nyxd_client.get_rewarded_set_nodes().await {
            Ok(rewarded_set) => rewarded_set,
            Err(err) => {
                warn!("failed to obtain the current rewarded set: {err}. falling back to the cached version");
                self.nym_contract_cache
                    .rewarded_set_owned()
                    .await
                    .into_inner()
                    .into()
            }
        };

        // we only need reward parameters for active set work factor and rewarded/active set sizes;
        // we do not need exact values of reward pool, staking supply, etc., so it's fine if it's slightly out of sync
        let Some(reward_params) = self
            .nym_contract_cache
            .interval_reward_params()
            .await
            .into_inner()
        else {
            error!("failed to obtain the current interval rewarding parameters. can't determine rewards without them");
            return Err(RewardingError::RewardingParamsRetrievalFailure);
        };

        Ok(self
            .load_nodes_for_rewarding(&interval, &rewarded_set, reward_params)
            .await)
    }
}
