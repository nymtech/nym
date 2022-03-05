// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// there is couple of reasons for putting this in a separate module:
// 1. I didn't feel it fit well in validator "cache". It seems like purpose of cache is to just keep updating local data
//    rather than attempting to change global view (i.e. the active set)
//
// 2. However, even if it was to exist in the validator cache refresher, we'd have to create a different "run"
//    method as it doesn't have access to the signing client which we need in the case of updating rewarded sets
//    (because validator cache can be run by anyone regardless of whether, say, network monitor exists)
//
// 3. Eventually this whole procedure is going to get expanded to allow for distribution of rewarded set generation
//    and hence this might be a good place for it.

use crate::contract_cache::ValidatorCache;
use crate::nymd_client::Client;
use crate::rewarding::error::RewardingError;
use crate::rewarding::MixnodeToReward;
use crate::storage::ValidatorApiStorage;
use mixnet_contract_common::reward_params::{IntervalRewardParams, NodeRewardParams, RewardParams};
use mixnet_contract_common::{IdentityKey, Interval, MixNodeBond};
use rand::prelude::SliceRandom;
use rand::rngs::OsRng;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::Notify;
use tokio::time::sleep;
use validator_client::nymd::SigningNymdClient;

pub(crate) struct FailedMixnodeRewardChunkDetails {
    possibly_unrewarded: Vec<MixnodeToReward>,
    error_message: String,
}

// Epoch has all the same semantics as interval, but has a lower set duration
type Epoch = Interval;

pub struct RewardedSetUpdater {
    nymd_client: Client<SigningNymdClient>,
    update_rewarded_set_notify: Arc<Notify>,
    validator_cache: ValidatorCache,
    storage: ValidatorApiStorage,
    epoch: Epoch,
}

impl RewardedSetUpdater {
    pub(crate) fn new(
        nymd_client: Client<SigningNymdClient>,
        update_rewarded_set_notify: Arc<Notify>,
        validator_cache: ValidatorCache,
        storage: ValidatorApiStorage,
    ) -> Self {
        let epoch = Epoch::new(0, OffsetDateTime::now_utc(), Duration::from_secs(3600));
        RewardedSetUpdater {
            nymd_client,
            update_rewarded_set_notify,
            validator_cache,
            storage,
            epoch,
        }
    }

    fn determine_rewarded_set(
        &self,
        mixnodes: Vec<MixNodeBond>,
        nodes_to_select: u32,
    ) -> Vec<IdentityKey> {
        if mixnodes.is_empty() {
            return Vec::new();
        }

        let mut rng = OsRng;

        // generate list of mixnodes and their relatively weight (by total stake)
        let choices = mixnodes
            .into_iter()
            .map(|mix| {
                // note that the theoretical maximum possible stake is equal to the total
                // supply of all tokens, i.e. 1B (which is 1 quadrillion of native tokens, i.e. 10^15 ~ 2^50)
                // which is way below maximum value of f64, so the cast is fine
                let total_stake = mix.total_bond().unwrap_or_default() as f64;
                (mix.mix_node.identity_key, total_stake)
            }) // if for some reason node is invalid, treat it as 0 stake/weight
            .collect::<Vec<_>>();

        // the unwrap here is fine as an error can only be thrown under one of the following conditions:
        // - our mixnode list is empty - we have already checked for that
        // - we have invalid weights, i.e. less than zero or NaNs - it shouldn't happen in our case as we safely cast down from u128
        // - all weights are zero - it's impossible in our case as the list of nodes is not empty and weight is proportional to stake. You must have non-zero stake in order to bond
        // - we have more than u32::MAX values (which is incredibly unrealistic to have 4B mixnodes bonded... literally every other person on the planet would need one)
        choices
            .choose_multiple_weighted(&mut rng, nodes_to_select as usize, |item| item.1)
            .unwrap()
            .map(|(identity, _weight)| identity.clone())
            .collect()
    }

    async fn reward_current_rewarded_set(
        &self,
        interval_reward_params: &IntervalRewardParams,
    ) -> Result<(), RewardingError> {
        let to_reward = self.nodes_to_reward(interval_reward_params).await?;
        let _failures = self.distribute_rewards(&to_reward, false);
        Ok(())
    }

    async fn distribute_rewards(
        &self,
        eligible_mixnodes: &[MixnodeToReward],
        retry: bool,
    ) -> Option<Vec<FailedMixnodeRewardChunkDetails>> {
        if retry {
            info!(
                "Attempting to retry rewarding {} mixnodes...",
                eligible_mixnodes.len()
            )
        } else {
            info!(
                "Attempting to reward {} mixnodes...",
                eligible_mixnodes.len()
            )
        }

        let mut failed_chunks = Vec::new();

        if let Err(err) = self
            .nymd_client
            .reward_mixnodes(eligible_mixnodes, self.epoch.id())
            .await
        {
            // this is a super weird edge case that we didn't catch change to sequence and
            // resent rewards unnecessarily, but the mempool saved us from executing it again
            // however, still we want to wait until we're sure we're into the next block
            if !err.is_tendermint_duplicate() {
                error!("failed to reward mixnodes... - {}", err);
                failed_chunks.push(FailedMixnodeRewardChunkDetails {
                    possibly_unrewarded: eligible_mixnodes.to_vec(),
                    error_message: err.to_string(),
                });
            }
            sleep(Duration::from_secs(11)).await;
        }

        let total_rewarded = eligible_mixnodes.len();
        info!("Rewarded {} mixndoes", total_rewarded);

        if failed_chunks.is_empty() {
            None
        } else {
            Some(failed_chunks)
        }
    }

    async fn nodes_to_reward(
        &self,
        interval_reward_params: &IntervalRewardParams,
    ) -> Result<Vec<MixnodeToReward>, RewardingError> {
        let active_set = self
            .validator_cache
            .active_set()
            .await
            .into_inner()
            .into_iter()
            .map(|bond| bond.mix_node.identity_key)
            .collect::<HashSet<_>>();

        let rewarded_set = self.validator_cache.rewarded_set().await.into_inner();

        // This is redundant the rewarded set already selects for the best nodes, so this feels quite wasteful gas wise
        // let mut nodes_with_delegations = Vec::with_capacity(all_nodes.len());
        // for node in all_nodes {
        //     let delegator_count = self
        //         .get_mixnode_delegators_count(node.mix_node.identity_key.clone())
        //         .await?;
        //     nodes_with_delegations.push((node, delegator_count));
        // }

        let mut eligible_nodes = Vec::with_capacity(rewarded_set.len());
        for rewarded_node in rewarded_set.into_iter() {
            let uptime = self
                .storage
                .get_average_mixnode_uptime_in_interval(
                    rewarded_node.identity(),
                    self.epoch.start_unix_timestamp(),
                    self.epoch.end_unix_timestamp(),
                )
                .await?;

            let node_reward_params = NodeRewardParams::new(
                0,
                uptime.u8().into(),
                active_set.contains(rewarded_node.identity()),
            );

            eligible_nodes.push(MixnodeToReward {
                identity: rewarded_node.identity().clone(),
                params: RewardParams::new(*interval_reward_params, node_reward_params),
            })
        }

        Ok(eligible_nodes)
    }

    // TODO: verify correctness without the rewarder running
    async fn update_rewarded_set(&self) -> Result<(), RewardingError> {
        // we know the entries are not stale, as a matter of fact they were JUST updated, since we got notified
        let all_nodes = self.validator_cache.mixnodes().await.into_inner();
        let interval_reward_params = self
            .validator_cache
            .interval_reward_params()
            .await
            .into_inner();
        // Reward all the nodes in the still current, soon to be previous rewarded set
        self.reward_current_rewarded_set(&interval_reward_params)
            .await?;

        let rewarded_set_size = interval_reward_params.rewarded_set_size() as u32;
        let active_set_size = interval_reward_params.active_set_size() as u32;

        // note that top k nodes are in the active set
        let new_rewarded_set = self.determine_rewarded_set(all_nodes, rewarded_set_size);
        if let Err(err) = self
            .nymd_client
            .write_rewarded_set(new_rewarded_set, active_set_size)
            .await
        {
            log::error!("failed to update the rewarded set - {}", err);
            // note that if the transaction failed to get executed because, I don't know, there was a networking hiccup
            // the cache will notify the updater on its next round
        }
        Ok(())
    }

    pub(crate) async fn run(&mut self) -> Result<(), RewardingError> {
        self.validator_cache.wait_for_initial_values().await;

        loop {
            // wait until the cache refresher determined its time to update the rewarded/active sets
            self.update_rewarded_set_notify.notified().await;
            self.epoch = self.epoch.next_interval();
            self.update_rewarded_set().await?;
        }
        #[allow(unreachable_code)]
        Ok(())
    }
}
