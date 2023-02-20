// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// there is couple of reasons for putting this in a separate module:
// 1. I didn't feel it fit well in nym contract "cache". It seems like purpose of cache is to just keep updating local data
//    rather than attempting to change global view (i.e. the active set)
//
// 2. However, even if it was to exist in the nym contract cache refresher, we'd have to create a different "run"
//    method as it doesn't have access to the signing client which we need in the case of updating rewarded sets
//    (because nym contract cache can be run by anyone regardless of whether, say, network monitor exists)
//
// 3. Eventually this whole procedure is going to get expanded to allow for distribution of rewarded set generation
//    and hence this might be a good place for it.

use crate::epoch_operations::helpers::stake_to_f64;
use crate::node_status_api::ONE_DAY;
use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::nyxd::Client;
use crate::support::storage::models::RewardingReport;
use crate::support::storage::NymApiStorage;
use error::RewardingError;
use nym_mixnet_contract_common::families::FamilyHead;
use nym_mixnet_contract_common::{
    reward_params::Performance, CurrentIntervalResponse, ExecuteMsg, Interval, MixId,
};
use nym_mixnet_contract_common::{IdentityKey, Layer, LayerAssignment, MixNodeDetails};
use nym_task::{TaskClient, TaskManager};
use rand::prelude::SliceRandom;
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::sleep;

pub(crate) mod error;
mod helpers;

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

pub struct RewardedSetUpdater {
    nyxd_client: Client,
    nym_contract_cache: NymContractCache,
    storage: NymApiStorage,
}

// Weight of a layer being chose is reciprocal to current count in layer
fn layer_weight(l: &Layer, layer_assignments: &HashMap<Layer, f32>) -> f32 {
    let total = layer_assignments.values().fold(0., |acc, i| acc + i);
    if total == 0. {
        1.
    } else {
        1. - (layer_assignments.get(l).unwrap_or(&0.) / total)
    }
}

impl RewardedSetUpdater {
    pub(crate) async fn current_interval_details(
        &self,
    ) -> Result<CurrentIntervalResponse, RewardingError> {
        Ok(self.nyxd_client.get_current_interval().await?)
    }

    pub(crate) fn new(
        nyxd_client: Client,
        nym_contract_cache: NymContractCache,
        storage: NymApiStorage,
    ) -> Self {
        RewardedSetUpdater {
            nyxd_client,
            nym_contract_cache,
            storage,
        }
    }

    async fn determine_layers(
        &self,
        rewarded_set: &[MixNodeDetails],
    ) -> Result<(Vec<LayerAssignment>, HashMap<String, Layer>), RewardingError> {
        let mut families_in_layer: HashMap<String, Layer> = HashMap::new();
        let mut assignments = vec![];
        let mut layer_assignments: HashMap<Layer, f32> = HashMap::new();
        let mut rng = OsRng;
        let layers = vec![Layer::One, Layer::Two, Layer::Three];

        let mix_to_family = self.nym_contract_cache.mix_to_family().await.to_vec();

        let mix_to_family = mix_to_family
            .into_iter()
            .collect::<HashMap<IdentityKey, FamilyHead>>();

        for mix in rewarded_set {
            let family = mix_to_family.get(&mix.bond_information.identity().to_owned());
            // Get layer already assigned to nodes family, if any
            let family_layer = family.and_then(|h| families_in_layer.get(h.identity()));

            // Same node families are always assigned to the same layer, otherwise layer selected by a random weighted choice
            let layer = if let Some(layer) = family_layer {
                layer.to_owned()
            } else {
                layers
                    .choose_weighted(&mut rng, |l| layer_weight(l, &layer_assignments))?
                    .to_owned()
            };

            assignments.push(LayerAssignment::new(mix.mix_id(), layer));

            // layer accounting
            let layer_entry = layer_assignments.entry(layer).or_insert(0.);
            *layer_entry += 1.;
            if let Some(family) = family {
                families_in_layer.insert(family.identity().to_string(), layer);
            }
        }

        Ok((assignments, families_in_layer))
    }

    fn determine_rewarded_set(
        &self,
        mixnodes: &[MixNodeDetails],
        nodes_to_select: u32,
    ) -> Result<Vec<MixNodeDetails>, RewardingError> {
        if mixnodes.is_empty() {
            return Ok(Vec::new());
        }

        let mut rng = OsRng;

        // generate list of mixnodes and their relatively weight (by total stake)
        let choices = mixnodes
            .iter()
            .map(|mix| {
                let total_stake = stake_to_f64(mix.total_stake());
                (mix.to_owned(), total_stake)
            })
            .collect::<Vec<_>>();

        // the unwrap here is fine as an error can only be thrown under one of the following conditions:
        // - our mixnode list is empty - we have already checked for that
        // - we have invalid weights, i.e. less than zero or NaNs - it shouldn't happen in our case as we safely cast down from u128
        // - all weights are zero - it's impossible in our case as the list of nodes is not empty and weight is proportional to stake. You must have non-zero stake in order to bond
        // - we have more than u32::MAX values (which is incredibly unrealistic to have 4B mixnodes bonded... literally every other person on the planet would need one)
        Ok(choices
            .choose_multiple_weighted(&mut rng, nodes_to_select as usize, |item| item.1)?
            .map(|(mix, _weight)| mix.to_owned())
            .collect())
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

    async fn update_rewarded_set_and_advance_epoch(
        &self,
        all_mixnodes: &[MixNodeDetails],
    ) -> Result<(), RewardingError> {
        // we grab rewarding parameters here as they might have gotten updated when performing epoch actions
        let rewarding_parameters = self.nyxd_client.get_current_rewarding_parameters().await?;

        let new_rewarded_set =
            self.determine_rewarded_set(all_mixnodes, rewarding_parameters.rewarded_set_size)?;

        let (layer_assignments, _families_in_layer) =
            self.determine_layers(&new_rewarded_set).await?;

        self.nyxd_client
            .advance_current_epoch(layer_assignments, rewarding_parameters.active_set_size)
            .await?;

        Ok(())
    }

    // This is where the epoch gets advanced, and all epoch related transactions originate
    async fn perform_epoch_operations(&self, interval: Interval) -> Result<(), RewardingError> {
        log::info!("The current epoch has finished.");
        log::info!(
            "Interval id: {}, epoch id: {} (absolute epoch id: {})",
            interval.current_interval_id(),
            interval.current_epoch_id(),
            interval.current_epoch_absolute_id()
        );
        log::info!(
            "The current epoch has lasted from {} until {}",
            interval.current_epoch_start(),
            interval.current_epoch_end()
        );

        log::info!("Performing all epoch operations...");

        let epoch_end = interval.current_epoch_end();

        let all_mixnodes = self.nym_contract_cache.mixnodes().await;
        if all_mixnodes.is_empty() {
            log::warn!("there don't seem to be any mixnodes on the network!")
        }

        // Reward all the nodes in the still current, soon to be previous rewarded set
        log::info!("Rewarding the current rewarded set...");
        if let Err(err) = self.reward_current_rewarded_set(interval).await {
            log::error!("FAILED to reward rewarded set - {err}");
            // since we haven't advanced the epoch yet, we will attempt to reward those nodes again
            // next time we enter this function (i.e. within 2min or so)
            //
            // TODO: deal with the following edge case:
            // - the nym api REWARDS all mixnodes
            // - then crashes before advancing epoch
            // - upon restart it will attempt (and fail) to re-reward the mixnodes
            return Err(err);
        } else {
            log::info!("Rewarded current rewarded set... SUCCESS");
        }

        // note: those operations don't really have to be atomic, so it's fine to send them
        // as separate transactions

        log::info!("Reconciling all pending epoch events...");
        if let Err(err) = self.nyxd_client.reconcile_epoch_events().await {
            log::error!("FAILED to reconcile epoch events... - {err}");
            return Err(err.into());
        } else {
            log::info!("Reconciled all pending epoch events... SUCCESS");
        }

        log::info!("Advancing epoch and updating the rewarded set...");
        if let Err(err) = self
            .update_rewarded_set_and_advance_epoch(&all_mixnodes)
            .await
        {
            log::error!("FAILED to advance the current epoch... - {err}");
            return Err(err);
        } else {
            log::info!("Advanced the epoch and updated the rewarded set... SUCCESS");
        }

        log::info!("Purging old node statuses from the storage...");
        let cutoff = (epoch_end - 2 * ONE_DAY).unix_timestamp();
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

        self.nym_contract_cache
            .update_mixnodes_blacklist(mix_blacklist_add, mix_blacklist_remove)
            .await;

        for gateway in gateways {
            if gateway.value() <= 50.0 {
                gate_blacklist_add.insert(gateway.identity().to_string());
            } else {
                gate_blacklist_remove.insert(gateway.identity().to_string());
            }
        }

        self.nym_contract_cache
            .update_gateways_blacklist(gate_blacklist_add, gate_blacklist_remove)
            .await;
        Ok(())
    }

    async fn wait_until_epoch_end(&mut self, shutdown: &mut TaskClient) -> Option<Interval> {
        const POLL_INTERVAL: Duration = Duration::from_secs(120);

        loop {
            let current_interval = match self.current_interval_details().await {
                Err(err) => {
                    error!("failed to obtain information about the current interval - {err}. Going to retry in {}s", POLL_INTERVAL.as_secs());
                    tokio::select! {
                        _ = sleep(POLL_INTERVAL) => {
                            continue
                        },
                        _ = shutdown.recv() => {
                            trace!("wait_until_epoch_end: Received shutdown");
                            break None
                        }
                    }
                }
                Ok(interval) => interval,
            };

            if current_interval.is_current_epoch_over {
                return Some(current_interval.interval);
            } else {
                let time_left = current_interval.time_until_current_epoch_end();
                log::info!(
                    "Waiting for epoch change, it should take approximately {}s",
                    time_left.as_secs()
                );
                let wait_time = if time_left < POLL_INTERVAL {
                    // add few seconds to adjust for possible block time drift
                    time_left + Duration::from_secs(10)
                } else {
                    POLL_INTERVAL
                };

                tokio::select! {
                    _ = sleep(wait_time) => {

                    },
                    _ = shutdown.recv() => {
                        trace!("wait_until_epoch_end: Received shutdown");
                        break None
                    }
                }
            }
        }
    }

    pub(crate) async fn run(&mut self, mut shutdown: TaskClient) -> Result<(), RewardingError> {
        self.nym_contract_cache.wait_for_initial_values().await;

        while !shutdown.is_shutdown() {
            let interval_details = match self.wait_until_epoch_end(&mut shutdown).await {
                // received a shutdown
                None => return Ok(()),
                Some(interval) => interval,
            };
            if let Err(err) = self.update_blacklist(&interval_details).await {
                error!("failed to update the node blacklist - {err}");
                continue;
            }
            if let Err(err) = self.perform_epoch_operations(interval_details).await {
                error!("failed to perform epoch operations - {err}");
                sleep(Duration::from_secs(30)).await;
            }
        }

        Ok(())
    }

    pub(crate) fn start(
        nyxd_client: Client,
        nym_contract_cache: &NymContractCache,
        storage: &NymApiStorage,
        shutdown: &TaskManager,
    ) {
        let mut rewarded_set_updater = RewardedSetUpdater::new(
            nyxd_client,
            nym_contract_cache.to_owned(),
            storage.to_owned(),
        );
        let shutdown_listener = shutdown.subscribe();
        tokio::spawn(async move { rewarded_set_updater.run(shutdown_listener).await });
    }
}

// before going any further, let's check whether we're allowed to perform rewarding
// (if not, let's blow up sooner rather than later)
pub(crate) async fn ensure_rewarding_permission(
    nyxd_client: &Client,
) -> Result<(), RewardingError> {
    let allowed_address = nyxd_client.get_rewarding_validator_address().await?;
    let our_address = nyxd_client.client_address().await;
    if allowed_address != our_address {
        Err(RewardingError::Unauthorised {
            our_address,
            allowed_address,
        })
    } else {
        Ok(())
    }
}
