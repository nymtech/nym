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
use crate::storage::models::RewardingReport;
use crate::storage::ValidatorApiStorage;
use mixnet_contract_common::reward_params::NodeRewardParams;
use mixnet_contract_common::ExecuteMsg;
use mixnet_contract_common::{IdentityKey, Interval, MixNodeBond};
use rand::prelude::SliceRandom;
use rand::rngs::OsRng;
use std::collections::HashSet;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::sleep;
use validator_client::nymd::{CosmosCoin, SigningNymdClient};

pub(crate) mod error;

use error::RewardingError;

#[derive(Debug, Clone)]
pub(crate) struct MixnodeToReward {
    pub(crate) identity: IdentityKey,

    /// Total number of individual addresses that have delegated to this particular node
    // pub(crate) total_delegations: usize,

    /// Node absolute uptime over total active set uptime
    pub(crate) params: NodeRewardParams,
}

impl MixnodeToReward {
    #[allow(dead_code)]
    fn params(&self) -> NodeRewardParams {
        self.params
    }

    #[allow(dead_code)]
    pub(crate) fn to_reward_execute_msg(&self) -> ExecuteMsg {
        ExecuteMsg::RewardMixnode {
            identity: self.identity.clone(),
            params: self.params(),
        }
    }
}

// Epoch has all the same semantics as interval, but has a lower set duration
type Epoch = Interval;

pub struct RewardedSetUpdater {
    nymd_client: Client<SigningNymdClient>,
    validator_cache: ValidatorCache,
    storage: ValidatorApiStorage,
}

impl RewardedSetUpdater {
    pub(crate) async fn epoch(&self) -> Result<Epoch, RewardingError> {
        Ok(self.nymd_client.get_current_epoch().await?)
    }

    pub(crate) async fn new(
        nymd_client: Client<SigningNymdClient>,
        validator_cache: ValidatorCache,
        storage: ValidatorApiStorage,
    ) -> Result<Self, RewardingError> {
        Ok(RewardedSetUpdater {
            nymd_client,
            validator_cache,
            storage,
        })
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
    ) -> Result<Vec<(ExecuteMsg, Vec<CosmosCoin>)>, RewardingError> {
        let to_reward = self.nodes_to_reward().await?;
        let epoch = self.epoch().await?;

        // self.storage.insert_started_epoch_rewarding(epoch).await?;

        let rewarding_report = RewardingReport {
            interval_rewarding_id: epoch.id() as i64,
            eligible_mixnodes: to_reward.len() as i64,
            possibly_unrewarded_mixnodes: 0,
        };

        self.storage
            .insert_rewarding_report(rewarding_report)
            .await?;

        self.generate_reward_messages(&to_reward).await
    }

    #[allow(unused_variables)]
    async fn generate_reward_messages(
        &self,
        eligible_mixnodes: &[MixnodeToReward],
    ) -> Result<Vec<(ExecuteMsg, Vec<CosmosCoin>)>, RewardingError> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "no-reward")] {
                Ok(vec![])
            } else {
                Ok(eligible_mixnodes
                    .iter()
                    .map(|node| node.to_reward_execute_msg())
                    .zip(std::iter::repeat(Vec::new()))
                    .collect())
            }
        }
    }

    async fn nodes_to_reward(&self) -> Result<Vec<MixnodeToReward>, RewardingError> {
        let epoch = self.epoch().await?;
        let active_set = self
            .validator_cache
            .active_set()
            .await
            .into_inner()
            .into_iter()
            .map(|bond| bond.mix_node.identity_key)
            .collect::<HashSet<_>>();

        let rewarded_set = self.validator_cache.rewarded_set().await.into_inner();

        let mut eligible_nodes = Vec::with_capacity(rewarded_set.len());
        for rewarded_node in rewarded_set.into_iter() {
            let uptime = self
                .storage
                .get_average_mixnode_uptime_in_the_last_24hrs(
                    rewarded_node.identity(),
                    epoch.end_unix_timestamp(),
                )
                .await?;

            let node_reward_params = NodeRewardParams::new(
                0,
                uptime.u8().into(),
                active_set.contains(rewarded_node.identity()),
            );

            eligible_nodes.push(MixnodeToReward {
                identity: rewarded_node.identity().clone(),
                params: node_reward_params,
            })
        }

        Ok(eligible_nodes)
    }

    // This is where the epoch gets advanced, and all epoch related transactions originate
    async fn update(&self) -> Result<(), RewardingError> {
        let epoch = self.epoch().await?;
        log::info!("Starting rewarded set update");
        // we know the entries are not stale, as a matter of fact they were JUST updated, since we got notified
        let all_nodes = self.validator_cache.mixnodes().await;
        let epoch_reward_params = self
            .validator_cache
            .epoch_reward_params()
            .await
            .into_inner();

        // Reward all the nodes in the still current, soon to be previous rewarded set
        // if let Err(err) = self.reward_current_rewarded_set().await {
        //     log::error!("FAILED to reward rewarded set - {}", err);
        // } else {
        //     log::info!("Rewarded current rewarded set... SUCCESS");
        // }

        let reward_msgs = self.reward_current_rewarded_set().await?;

        let rewarded_set_size = epoch_reward_params.rewarded_set_size() as u32;
        let active_set_size = epoch_reward_params.active_set_size() as u32;

        // note that top k nodes are in the active set
        let new_rewarded_set = self.determine_rewarded_set(all_nodes, rewarded_set_size);

        if let Err(err) = self
            .nymd_client
            .epoch_operations(new_rewarded_set, active_set_size, reward_msgs)
            .await
        {
            log::error!("FAILED epoch operations - {}", err);
        } else {
            log::info!("Epoch operations... SUCCESS");
        }

        let cutoff = (epoch.end() - Duration::from_secs(86400)).unix_timestamp();
        self.storage.purge_old_statuses(cutoff).await?;

        Ok(())
    }

    async fn update_blacklist(&mut self, epoch: &Interval) -> Result<(), RewardingError> {
        info!("Updating blacklist");

        let mut mix_blacklist_add = HashSet::new();
        let mut mix_blacklist_remove = HashSet::new();
        let mut gate_blacklist_add = HashSet::new();
        let mut gate_blacklist_remove = HashSet::new();

        let mixnodes = self
            .storage
            .get_all_avg_mix_reliability_in_last_24hr(epoch.end_unix_timestamp())
            .await?;
        let gateways = self
            .storage
            .get_all_avg_gateway_reliability_in_last_24hr(epoch.end_unix_timestamp())
            .await?;

        // TODO: Make thresholds configurable
        for mix in mixnodes {
            if mix.value() <= 50.0 {
                mix_blacklist_add.insert(mix.identity().to_string());
            } else {
                mix_blacklist_remove.insert(mix.identity().to_string());
            }
        }

        self.validator_cache
            .update_mixnodes_blacklist(mix_blacklist_add, mix_blacklist_remove)
            .await;

        for gateway in gateways {
            if gateway.value() <= 50.0 {
                gate_blacklist_add.insert(gateway.identity().to_string());
            } else {
                gate_blacklist_remove.insert(gateway.identity().to_string());
            }
        }

        self.validator_cache
            .update_gateways_blacklist(gate_blacklist_add, gate_blacklist_remove)
            .await;
        Ok(())
    }

    pub(crate) async fn run(&mut self) -> Result<(), RewardingError> {
        self.validator_cache.wait_for_initial_values().await;

        loop {
            // wait until the cache refresher determined its time to update the rewarded/active sets
            let time = OffsetDateTime::now_utc().unix_timestamp();
            let epoch = self.epoch().await?;
            let time_to_epoch_change = epoch.end_unix_timestamp() - time;
            if time_to_epoch_change <= 0 {
                self.update_blacklist(&epoch).await?;
                log::info!(
                    "Time to epoch change is {}, updating rewarded set",
                    time_to_epoch_change
                );
                self.update().await?;
            } else {
                log::info!(
                    "Waiting for epoch change, time to epoch change is {}",
                    time_to_epoch_change
                );
                // Sleep at most 300 before checking again, to keep logs busy
                let s = time_to_epoch_change.min(300).max(0) as u64;
                sleep(Duration::from_secs(s)).await;
            }
            // allow some blocks to pass
            sleep(Duration::from_secs(10)).await;
        }
        #[allow(unreachable_code)]
        Ok(())
    }
}
