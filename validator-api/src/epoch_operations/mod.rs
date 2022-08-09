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
use mixnet_contract_common::{
    reward_params::Performance, CurrentIntervalResponse, ExecuteMsg, Interval, NodeId,
};
use rand::prelude::SliceRandom;
use rand::rngs::OsRng;
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::sleep;
use validator_client::nymd::SigningNymdClient;

pub(crate) mod error;
mod helpers;

use crate::epoch_operations::helpers::stake_to_f64;
use error::RewardingError;
use mixnet_contract_common::mixnode::MixNodeDetails;

#[derive(Debug, Clone, Copy)]
pub(crate) struct MixnodeToReward {
    pub(crate) mix_id: NodeId,

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

// // Epoch has all the same semantics as interval, but has a lower set duration
// type Epoch = Interval;

pub struct RewardedSetUpdater {
    nymd_client: Client<SigningNymdClient>,
    validator_cache: ValidatorCache,
    storage: ValidatorApiStorage,
}

impl RewardedSetUpdater {
    pub(crate) async fn current_interval_details(
        &self,
    ) -> Result<CurrentIntervalResponse, RewardingError> {
        Ok(self.nymd_client.get_current_interval().await?)
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
        mixnodes: Vec<MixNodeDetails>,
        nodes_to_select: u32,
    ) -> Vec<NodeId> {
        if mixnodes.is_empty() {
            return Vec::new();
        }

        let mut rng = OsRng;

        // generate list of mixnodes and their relatively weight (by total stake)
        let choices = mixnodes
            .into_iter()
            .map(|mix| {
                let total_stake = stake_to_f64(mix.total_stake());
                (mix.mix_id(), total_stake)
            })
            .collect::<Vec<_>>();

        // the unwrap here is fine as an error can only be thrown under one of the following conditions:
        // - our mixnode list is empty - we have already checked for that
        // - we have invalid weights, i.e. less than zero or NaNs - it shouldn't happen in our case as we safely cast down from u128
        // - all weights are zero - it's impossible in our case as the list of nodes is not empty and weight is proportional to stake. You must have non-zero stake in order to bond
        // - we have more than u32::MAX values (which is incredibly unrealistic to have 4B mixnodes bonded... literally every other person on the planet would need one)
        choices
            .choose_multiple_weighted(&mut rng, nodes_to_select as usize, |item| item.1)
            .unwrap()
            .map(|(mix_id, _weight)| *mix_id)
            .collect()
    }

    async fn reward_current_rewarded_set(
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

        if let Err(err) = self.nymd_client.send_rewarding_messages(&to_reward).await {
            error!(
                "failed to perform mixnode rewarding for epoch {}! Error encountered: {}",
                current_interval.current_epoch_absolute_id(),
                err
            );
            return Err(err.into());
        }

        log::info!("rewarded {} mixnodes...", to_reward.len());

        let rewarding_report = RewardingReport {
            absolute_epoch_id: current_interval.current_epoch_absolute_id(),
            eligible_mixnodes: to_reward.len() as u32,
            possibly_unrewarded_mixnodes: 0,
        };

        self.storage
            .insert_rewarding_report(rewarding_report)
            .await?;

        Ok(())
    }

    async fn nodes_to_reward(&self, interval: Interval) -> Vec<MixnodeToReward> {
        let rewarded_set = self
            .validator_cache
            .rewarded_set_detailed()
            .await
            .into_inner();

        let mut eligible_nodes = Vec::with_capacity(rewarded_set.len());
        for mixnode in rewarded_set {
            let uptime = self
                .storage
                .get_average_mixnode_uptime_in_the_last_24hrs(
                    mixnode.mix_id(),
                    interval.current_epoch_end_unix_timestamp(),
                )
                .await
                .unwrap_or_default();
            eligible_nodes.push(MixnodeToReward {
                mix_id: mixnode.mix_id(),
                performance: uptime.into(),
            })
        }

        eligible_nodes
    }

    async fn update_rewarded_set_and_advance_epoch(
        &self,
        all_mixnodes: Vec<MixNodeDetails>,
    ) -> Result<(), RewardingError> {
        // we grab rewarding parameters here as they might have gotten updated when performing epoch actions
        let rewarding_parameters = self.nymd_client.get_current_rewarding_parameters().await?;

        let new_rewarded_set =
            self.determine_rewarded_set(all_mixnodes, rewarding_parameters.rewarded_set_size);
        self.nymd_client
            .advance_current_epoch(new_rewarded_set, rewarding_parameters.active_set_size)
            .await?;

        Ok(())
    }

    // This is where the epoch gets advanced, and all epoch related transactions originate
    async fn perform_epoch_operations(&self, interval: Interval) -> Result<(), RewardingError> {
        log::info!("The current epoch has finished. Performing all epoch operations...");

        let epoch_end = interval.current_epoch_end();

        let all_nodes = self.validator_cache.mixnodes().await;
        if all_nodes.is_empty() {
            log::warn!("there don't seem to be any mixnodes on the network!")
        }

        // get list of all mixnodes BEFORE rewarding happens as to now be biased by rewards
        // that might be given to them
        let all_mixnodes = self.validator_cache.mixnodes().await;

        // Reward all the nodes in the still current, soon to be previous rewarded set
        log::info!("Rewarding the current rewarded set...");
        if let Err(err) = self.reward_current_rewarded_set(interval).await {
            log::error!("FAILED to reward rewarded set - {}", err);
            // since we haven't advanced the epoch yet, we will attempt to reward those nodes again
            // next time we enter this function (i.e. within 2min or so)
            return Err(err);
        } else {
            log::info!("Rewarded current rewarded set... SUCCESS");
        }

        // note: those operations don't really have to be atomic, so it's fine to send them
        // as separate transactions

        log::info!("Reconciling all pending epoch events...");
        if let Err(err) = self.nymd_client.reconcile_epoch_events().await {
            log::error!("FAILED to reconcile epoch events... - {}", err);
            return Err(err.into());
        } else {
            log::info!("Reconciled all pending epoch events... SUCCESS");
        }

        log::info!("Advancing epoch and updating the rewarded set...");
        if let Err(err) = self
            .update_rewarded_set_and_advance_epoch(all_mixnodes)
            .await
        {
            log::error!("FAILED to advance the current epoch... - {}", err);
            return Err(err);
        } else {
            log::info!("Advanced the epoch and updated the rewarded set... SUCCESS");
        }

        log::info!("Puring all node statuses from the storage...");
        let cutoff = (epoch_end - Duration::from_secs(86400)).unix_timestamp();
        self.storage.purge_old_statuses(cutoff).await?;

        Ok(())
    }

    async fn update_blacklist(&mut self, interval: &Interval) -> Result<(), RewardingError> {
        info!("Updating blacklists");

        let mut mix_blacklist_add = HashSet::new();
        let mut mix_blacklist_remove = HashSet::new();
        let mut gate_blacklist_add = HashSet::new();
        let mut gate_blacklist_remove = HashSet::new();

        let mixnodes = self
            .storage
            .get_all_avg_mix_reliability_in_last_24hr(interval.current_epoch_end_unix_timestamp())
            .await?;
        let gateways = self
            .storage
            .get_all_avg_gateway_reliability_in_last_24hr(
                interval.current_epoch_end_unix_timestamp(),
            )
            .await?;

        // TODO: Make thresholds configurable
        for mix in mixnodes {
            if mix.value() <= 50.0 {
                mix_blacklist_add.insert(mix.mix_id());
            } else {
                mix_blacklist_remove.insert(mix.mix_id());
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

    async fn wait_until_epoch_end(&mut self) -> Interval {
        const POLL_INTERVAL: Duration = Duration::from_secs(120);

        loop {
            let current_interval = match self.current_interval_details().await {
                Err(err) => {
                    error!("failed to obtain information about the current interval - {}. Going to retry in {}s", err, POLL_INTERVAL.as_secs());
                    sleep(POLL_INTERVAL).await;
                    continue;
                }
                Ok(interval) => interval,
            };

            if current_interval.is_current_epoch_over {
                return current_interval.interval;
            } else {
                let time_left = current_interval.time_until_current_epoch_end();
                log::info!(
                    "Waiting for epoch change, it should take approximately {}s",
                    time_left.as_secs()
                );
                if time_left < POLL_INTERVAL {
                    // add few seconds to adjust for possible block time drift
                    sleep(time_left + Duration::from_secs(10)).await
                } else {
                    sleep(POLL_INTERVAL).await;
                }
            }
        }
    }

    pub(crate) async fn run(&mut self) -> Result<(), RewardingError> {
        self.validator_cache.wait_for_initial_values().await;

        loop {
            let interval_details = self.wait_until_epoch_end().await;
            if let Err(err) = self.update_blacklist(&interval_details).await {
                error!("failed to update the node blacklist - {}", err);
                continue;
            }
            if let Err(err) = self.perform_epoch_operations(interval_details).await {
                error!("failed to perform epoch operations - {}", err)
            }
        }
    }
}
