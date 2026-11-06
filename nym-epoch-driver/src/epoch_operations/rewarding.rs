// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::epoch_operations::EpochAdvancer;
use crate::epoch_operations::helpers::{RewardedNodeWithParams, rewarded_nodes_with_work};
use anyhow::bail;
use nym_mixnet_contract_common::{EpochState, ExecuteMsg};
use nym_validator_client::nyxd::Coin;
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;
use nym_validator_client::nyxd::contract_traits::mixnet_query_client::MixnetQueryClientExt;
use tracing::{error, info, warn};

impl EpochAdvancer {
    pub(super) async fn reward_current_rewarded_set(&self) -> anyhow::Result<()> {
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

                match self._reward_current_rewarded_set().await {
                    Ok(rewarded) => {
                        info!("Rewarded {rewarded} nodes of the current rewarded set... SUCCESS");
                        Ok(())
                    }
                    Err(err) => {
                        error!("FAILED to reward the current rewarded set: {err}");
                        Err(err)
                    }
                }
            }
        }
    }

    /// Pays every node of the rewarded set; returns how many were paid.
    async fn _reward_current_rewarded_set(&self) -> anyhow::Result<usize> {
        let rewarded_set = self.nyxd_client.get_rewarded_set().await?;
        let reward_params = self.nyxd_client.get_rewarding_parameters().await?;

        let mut rewards =
            rewarded_nodes_with_work(&rewarded_set.assignment, reward_params, &self.window);
        rewards.sort_by_key(|node| node.node_id);

        // the contract enters the rewarding state only with a non-empty rewarded set
        self.send_rewarding_messages(&rewards).await?;
        Ok(rewards.len())
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
