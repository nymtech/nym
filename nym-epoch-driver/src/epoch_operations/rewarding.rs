// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::epoch_operations::EpochAdvancer;
use crate::epoch_operations::helpers::RewardedNodeWithParams;
use anyhow::bail;
use nym_mixnet_contract_common::{EpochState, ExecuteMsg, Interval};
use nym_validator_client::nyxd::Coin;
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;
use nym_validator_client::nyxd::contract_traits::mixnet_query_client::MixnetQueryClientExt;
use tracing::{error, info, warn};

impl EpochAdvancer {
    pub(super) async fn reward_current_rewarded_set(
        &self,
        to_reward: Vec<RewardedNodeWithParams>,
        current_interval: Interval,
    ) -> anyhow::Result<()> {
        let epoch_status = self.nyxd_client.get_current_epoch_status().await?;
        match epoch_status.state {
            EpochState::InProgress => {
                // hard error, this shouldn't have happened!
                error!("tried to perform node rewarding while the epoch is still in progress!");
                bail!(
                    "the current epoch is in the wrong state ({}) to perform the requested operation: mix rewarding",
                    EpochState::InProgress
                )
            }
            EpochState::ReconcilingEvents | EpochState::RoleAssignment { .. } => {
                warn!(
                    "we seem to have crashed mid epoch operations... no need to reward nodes as we've already done that! (or this could be a false positive if there were no nodes to reward - to fix this warning later)"
                );
                Ok(())
            }
            EpochState::Rewarding { last_rewarded, .. } => {
                info!("Rewarding the current rewarded set...");

                // with how the driver is currently coded, this should never happen as we're always
                // rewarding ALL nodes at once, but who knows what we might decide to do in the future...
                if last_rewarded != 0 {
                    bail!(
                        "it seems the current epoch is in mid-rewarding state (last rewarded is {last_rewarded}). With our current driver this shouldn't have been possible. Manual intervention is required."
                    )
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
    ) -> anyhow::Result<()> {
        if to_reward.is_empty() {
            error!(
                "There are no nodes to reward in this epoch - we shouldn't have been in the 'Rewarding' state!"
            );
        } else if let Err(err) = self.send_rewarding_messages(&to_reward).await {
            error!(
                "failed to perform node rewarding for epoch {}! Error encountered: {err}",
                current_interval.current_epoch_absolute_id(),
            );
            return Err(err);
        }

        info!("rewarded {} nodes...", to_reward.len());

        Ok(())
    }

    async fn send_rewarding_messages(
        &self,
        rewarded_set: &[RewardedNodeWithParams],
    ) -> anyhow::Result<()> {
        let mixnet_contract = self.mixnet_contract()?;
        let msgs = generate_reward_messages(rewarded_set);

        self.nyxd_client
            .execute_multiple(
                &mixnet_contract,
                msgs,
                None,
                format!("rewarding {} nodes", rewarded_set.len()),
            )
            .await?;
        Ok(())
    }

    pub(crate) async fn nodes_to_reward(&self) -> anyhow::Result<Vec<RewardedNodeWithParams>> {
        let rewarded_set = self.nyxd_client.get_rewarded_set().await?;
        let reward_params = self.nyxd_client.get_rewarding_parameters().await?;

        Ok(self.load_nodes_for_rewarding(&rewarded_set, reward_params))
    }
}

fn generate_reward_messages(
    rewarded_set: &[RewardedNodeWithParams],
) -> Vec<(ExecuteMsg, Vec<Coin>)> {
    rewarded_set
        .iter()
        .map(|node| (*node).into())
        .zip(std::iter::repeat(Vec::new()))
        .collect()
}
