// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract_cache::ValidatorCache;
use crate::node_status_api::ONE_DAY;
use crate::nymd_client::Client;
use crate::rewarding::error::RewardingError;
use crate::storage::models::{
    FailedMixnodeRewardChunk, PossiblyUnrewardedMixnode, RewardingReport,
};
use crate::storage::ValidatorApiStorage;
use config::defaults::DEFAULT_NETWORK;
use log::{error, info};
use mixnet_contract_common::reward_params::{IntervalRewardParams, NodeRewardParams, RewardParams};
use mixnet_contract_common::{
    ExecuteMsg, IdentityKey, Interval, MixNodeBond, RewardingStatus, MIXNODE_DELEGATORS_PAGE_LIMIT,
};
use std::collections::HashSet;
use std::convert::TryInto;
use std::process;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::sleep;
use validator_client::nymd::SigningNymdClient;

pub(crate) mod error;
#[derive(Debug, Clone)]
pub(crate) struct MixnodeToReward {
    pub(crate) identity: IdentityKey,

    /// Total number of individual addresses that have delegated to this particular node
    // pub(crate) total_delegations: usize,

    /// Node absolute uptime over total active set uptime
    pub(crate) params: RewardParams,
}

impl MixnodeToReward {
    fn params(&self) -> RewardParams {
        self.params
    }
}

impl MixnodeToReward {
    pub(crate) fn to_reward_execute_msg(&self, interval_id: u32) -> ExecuteMsg {
        ExecuteMsg::RewardMixnode {
            identity: self.identity.clone(),
            params: self.params(),
            interval_id,
        }
    }

    // pub(crate) fn to_next_delegator_reward_execute_msg(&self, interval_id: u32) -> ExecuteMsg {
    //     ExecuteMsg::RewardNextMixDelegators {
    //         mix_identity: self.identity.clone(),
    //         interval_id,
    //     }
    // }
}

pub(crate) struct FailedMixnodeRewardChunkDetails {
    possibly_unrewarded: Vec<MixnodeToReward>,
    error_message: String,
}

#[derive(Default)]
pub(crate) struct FailureData {
    mixnodes: Option<Vec<FailedMixnodeRewardChunkDetails>>,
}

pub(crate) struct Rewarder {
    nymd_client: Client<SigningNymdClient>,
    validator_cache: ValidatorCache,
    storage: ValidatorApiStorage,

    /// Ideal world, expected number of network monitor test runs per interval.
    /// In reality it will be slightly lower due to network delays, but it's good enough
    /// for estimations regarding percentage of available data for reward distribution.
    expected_interval_monitor_runs: usize,

    /// Minimum percentage of network monitor test runs reports required in order to distribute
    /// rewards.
    minimum_interval_monitor_threshold: u8,
}

impl Rewarder {
    pub(crate) fn new(
        nymd_client: Client<SigningNymdClient>,
        validator_cache: ValidatorCache,
        storage: ValidatorApiStorage,
        expected_interval_monitor_runs: usize,
        minimum_interval_monitor_threshold: u8,
    ) -> Self {
        Rewarder {
            nymd_client,
            validator_cache,
            storage,
            expected_interval_monitor_runs,
            minimum_interval_monitor_threshold,
        }
    }

    /// Obtains the current number of delegators that have delegated their stake towards this particular mixnode.
    ///
    /// # Arguments
    ///
    /// * `identity`: identity key of the mixnode
    async fn get_mixnode_delegators_count(
        &self,
        identity: IdentityKey,
    ) -> Result<usize, RewardingError> {
        Ok(self
            .nymd_client
            .get_mixnode_delegations(identity)
            .await?
            .len())
    }

    /// Obtain the list of current 'rewarded' set, determine their uptime in the provided interval
    /// and attach information required for rewarding.
    ///
    /// The method also obtains the number of delegators towards the node in order to more accurately
    /// approximate the required gas fees when distributing the rewards.
    ///
    /// # Arguments
    ///
    /// * `interval`: current rewarding interval
    async fn determine_eligible_mixnodes(
        &self,
        interval: Interval,
    ) -> Result<Vec<MixnodeToReward>, RewardingError> {
        let interval_reward_params = self
            .nymd_client
            .get_current_epoch_reward_params()
            .await?;

        info!("Rewarding pool stats");
        info!(
            "---- Interval reward pool: {} {}",
            interval_reward_params.period_reward_pool(),
            DEFAULT_NETWORK.denom()
        );
        info!(
            "-- Circulating supply: {} {}",
            interval_reward_params.circulating_supply(),
            DEFAULT_NETWORK.denom()
        );

        // 1. get list of all currently bonded nodes
        // 2. for each of them determine their delegator count
        // 3. for each of them determine their uptime for the interval
        // 4. for each of them determine if they're currently in the active set
        // TODO: step 4 will definitely need to change.
        // let all_nodes = self.validator_cache.mixnodes().await.into_inner();
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
                    interval.start_unix_timestamp(),
                    interval.end_unix_timestamp(),
                )
                .await?;

            let node_reward_params = NodeRewardParams::new(
                0,
                uptime.u8().into(),
                active_set.contains(rewarded_node.identity()),
            );

            eligible_nodes.push(MixnodeToReward {
                identity: rewarded_node.identity().clone(),
                params: RewardParams::new(interval_reward_params, node_reward_params),
            })
        }

        Ok(eligible_nodes)
    }

    /// Check whether every node, and their delegators, on the provided list were fully rewarded
    /// in the specified interval.
    ///
    /// It is used to deal with edge cases such that mixnode had exactly full page of delegations and
    /// somebody created a new delegation thus causing the "last" delegator to possibly be pushed
    /// onto the next page that the validator API was not aware of.
    ///
    /// * `eligible_mixnodes`: list of the nodes that were eligible to receive rewards.
    /// * `interval_id`: nonce associated with the current rewarding interval
    async fn verify_rewarding_completion(
        &self,
        eligible_mixnodes: &[MixnodeToReward],
        current_rewarding_nonce: u32,
    ) -> (Vec<MixnodeToReward>, Vec<MixnodeToReward>) {
        let mut unrewarded = Vec::new();
        let mut further_delegators_present = Vec::new();
        for mix in eligible_mixnodes {
            match self
                .nymd_client
                .get_rewarding_status(mix.identity.clone(), current_rewarding_nonce)
                .await
            {
                Ok(rewarding_status) => match rewarding_status.status {
                    // that case is super weird, it implies the node hasn't been rewarded at all!
                    // maybe the transaction timed out twice or something? In any case, we should attempt
                    // the reward for the final time!
                    None => unrewarded.push(mix.clone()),
                    Some(RewardingStatus::PendingNextDelegatorPage(_)) => {
                        further_delegators_present.push(mix.clone())
                    }
                    Some(RewardingStatus::Complete(_)) => {}
                },
                Err(err) => {
                    error!(
                        "failed to query rewarding status of {} - {}",
                        mix.identity, err
                    )
                }
            }
        }
        (unrewarded, further_delegators_present)
    }

    // Utility function to print to the stdout rewarding progress
    fn print_rewarding_progress(&self, total_rewarded: usize, out_of: usize, is_retry: bool) {
        let percentage = total_rewarded as f32 * 100.0 / out_of as f32;

        if is_retry {
            info!(
                "Resent rewarding transaction for {} / {} mixnodes\t{:.2}%",
                total_rewarded, out_of, percentage
            );
        } else {
            info!(
                "Sent rewarding transaction for {} / {} mixnodes\t{:.2}%",
                total_rewarded, out_of, percentage
            );
        }
    }

    // FIXME: Remove dead code
    /// Using the list of mixnodes eligible for rewards, chunks it into pre-defined sized-chunks
    /// and gives out the rewards by calling the smart contract.
    ///
    /// Returns an optional vector containing list of chunks that experienced a smart contract
    /// execution error during reward distribution. However, it does not necessarily imply they
    /// were not rewarded. There are some edge cases where we time out waiting for block to be included
    /// yet the transactions went through.
    ///
    /// Only returns errors for problems originating from before smart contract was called, i.e.
    /// we know for sure not a single node has been rewarded.
    ///
    /// # Arguments
    ///
    /// * `eligible_mixnodes`: list of the nodes that are eligible to receive rewards.
    /// * `interval_id`: nonce associated with the current rewarding interval.
    /// * `retry`: flag to indicate whether this is a retry attempt for rewarding particular nodes.
    async fn distribute_rewards_to_mixnodes(
        &self,
        eligible_mixnodes: &[MixnodeToReward],
        interval_id: u32,
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

        // construct chunks such that we reward at most MIXNODE_DELEGATORS_PAGE_LIMIT delegators per block

        // nodes with > MIXNODE_DELEGATORS_PAGE_LIMIT delegators that have to be treated in a special way,
        // because we cannot batch them together
        // let mut individually_rewarded = Vec::new();

        // sets of nodes that together they have < MIXNODE_DELEGATORS_PAGE_LIMIT delegators
        // let mut batch_rewarded = vec![vec![]];
        // let mut current_batch_i = 0;
        // let mut current_batch_total = 0;

        // right now put mixes into batches super naively, if it doesn't fit into the current one,
        // create a new one.
        // for mix in eligible_mixnodes {
        //     batch_rewarded[current_batch_i].push(mix.clone());
        //     // if mixnode has uptime of 0, no rewarding will actually happen regardless of number of delegators,
        //     // so we can just batch it with the current batch
        //     if mix.params.uptime() == 0 {
        //         batch_rewarded[current_batch_i].push(mix.clone());
        //         continue;
        //     }

        //     if mix.total_delegations > MIXNODE_DELEGATORS_PAGE_LIMIT {
        //         individually_rewarded.push(mix)
        //     } else if current_batch_total + mix.total_delegations < MIXNODE_DELEGATORS_PAGE_LIMIT {
        //         batch_rewarded[current_batch_i].push(mix.clone());
        //         current_batch_total += mix.total_delegations;
        //     } else {
        //         batch_rewarded.push(vec![mix.clone()]);
        //         current_batch_i += 1;
        //         current_batch_total = 0;
        //     }
        // }

        // start rewarding, first the nodes that are dealt with individually, i.e. nodes that
        // need to have their own special blocks due to number of delegators
        // for mix in individually_rewarded {
        //     if let Err(err) = self
        //         .nymd_client
        //         .reward_mixnode_and_all_delegators(mix, interval_id)
        //         .await
        //     {
        //         // this is a super weird edge case that we didn't catch change to sequence and
        //         // resent rewards unnecessarily, but the mempool saved us from executing it again
        //         // however, still we want to wait until we're sure we're into the next block
        //         if !err.is_tendermint_duplicate() {
        //             error!("failed to reward mixnode with all delegators... - {}", err);
        //             failed_chunks.push(FailedMixnodeRewardChunkDetails {
        //                 possibly_unrewarded: vec![mix.clone()],
        //                 error_message: err.to_string(),
        //             });
        //         }
        //         sleep(Duration::from_secs(11)).await;
        //     }

        //     total_rewarded += 1;
        //     self.print_rewarding_progress(total_rewarded, eligible_mixnodes.len(), retry);
        // }

        if let Err(err) = self
            .nymd_client
            .reward_mixnodes(eligible_mixnodes, interval_id)
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
        self.print_rewarding_progress(total_rewarded, eligible_mixnodes.len(), retry);

        // then we move onto the chunks
        // for mix_chunk in batch_rewarded {
        //     if mix_chunk.is_empty() {
        //         continue;
        //     }
        //     if let Err(err) = self
        //         .nymd_client
        //         .reward_mixnodes_with_single_page_of_delegators(&mix_chunk, interval_id)
        //         .await
        //     {
        //         // this is a super weird edge case that we didn't catch change to sequence and
        //         // resent rewards unnecessarily, but the mempool saved us from executing it again
        //         // however, still we want to wait until we're sure we're into the next block
        //         if !err.is_tendermint_duplicate() {
        //             error!("failed to reward mixnodes... - {}", err);
        //             failed_chunks.push(FailedMixnodeRewardChunkDetails {
        //                 possibly_unrewarded: mix_chunk.to_vec(),
        //                 error_message: err.to_string(),
        //             });
        //         }
        //         sleep(Duration::from_secs(11)).await;
        //     }

        //     total_rewarded += mix_chunk.len();
        //     self.print_rewarding_progress(total_rewarded, eligible_mixnodes.len(), retry);
        // }

        if failed_chunks.is_empty() {
            None
        } else {
            Some(failed_chunks)
        }
    }

    // FIXME: Delete after refactoring is done
    /// For each mixnode on the list, try to "continue" rewarding its delegators.
    /// Note: due to the checks inside the smart contract, it's impossible to accidentally
    /// reward the same mixnode (or delegator) twice during particular rewarding interval.
    ///
    /// Realistically if this method is ever called, it will be only done once per node, so there's
    /// no need to determine the exact number of missed delegators.
    ///
    /// * `nodes`: mixnodes which delegators did not receive all rewards in this interval.
    /// * `interval_id`: nonce associated with the current rewarding interval.
    // async fn reward_missed_delegators(&self, nodes: &[MixnodeToReward], interval_id: u32) {
    //     let mut total_resent = 0;
    //     for missed_node in nodes {
    //         total_resent += 1;
    //         info!(
    //             "Sending rewarding transaction for missed delegators ({} / {} mixnodes re-checked)",
    //             total_resent,
    //             nodes.len()
    //         );

    //         if let Err(err) = self
    //             .nymd_client
    //             .reward_mix_delegators(missed_node, interval_id)
    //             .await
    //         {
    //             warn!(
    //                 "failed to attempt to reward missed delegators of node {} - {}",
    //                 missed_node.identity, err
    //             )
    //         }
    //     }
    // }

    /// Using the list of active mixnode and gateways, determine which of them are eligible for
    /// rewarding and distribute the rewards.
    ///
    /// # Arguments
    ///
    /// * `interval_rewarding_id`: id of the current interval rewarding as stored in the database.
    /// * `interval`: current rewarding interval
    async fn distribute_rewards(
        &self,
        interval_rewarding_database_id: i64,
        interval: Interval,
    ) -> Result<(RewardingReport, Option<FailureData>), RewardingError> {
        let mut failure_data = FailureData::default();

        let eligible_mixnodes = self.determine_eligible_mixnodes(interval).await?;
        if eligible_mixnodes.is_empty() {
            return Err(RewardingError::NoMixnodesToReward);
        }
        let total_eligible = eligible_mixnodes.len();

        failure_data.mixnodes = self
            .distribute_rewards_to_mixnodes(&eligible_mixnodes, interval.id(), false)
            .await;

        let mut nodes_to_verify = eligible_mixnodes;

        // if there's some underlying networking error or something, don't keep retrying forever
        let mut retries_allowed = 5;
        loop {
            if retries_allowed <= 0 {
                break;
            }
            let (unrewarded, mut pending_delegators) = self
                .verify_rewarding_completion(&nodes_to_verify, interval.id())
                .await;
            if unrewarded.is_empty() && pending_delegators.is_empty() {
                // we're all good - everyone got their rewards
                break;
            }

            if !unrewarded.is_empty() {
                // no need to save failure data as we already know about those from the very first run
                self.distribute_rewards_to_mixnodes(&unrewarded, interval.id(), true)
                    .await;
            }

            // FIXME: Delete after refactoring is done
            // if !pending_delegators.is_empty() {
            //     self.reward_missed_delegators(&pending_delegators, interval.id())
            //         .await;
            // }

            // no point in verifying EVERYTHING again, just check the nodes that went through retries
            nodes_to_verify = unrewarded;
            nodes_to_verify.append(&mut pending_delegators);

            retries_allowed -= 1;
        }

        let report = RewardingReport {
            interval_rewarding_id: interval_rewarding_database_id,
            eligible_mixnodes: total_eligible as i64,
            possibly_unrewarded_mixnodes: failure_data
                .mixnodes
                .as_ref()
                .map(|chunks| {
                    chunks
                        .iter()
                        .map(|chunk| chunk.possibly_unrewarded.len())
                        .sum::<usize>() as i64
                })
                .unwrap_or_default(),
        };

        self.nymd_client.advance_current_interval().await?;

        if failure_data.mixnodes.is_none() {
            Ok((report, None))
        } else {
            Ok((report, Some(failure_data)))
        }
    }

    /// Saves information about possibly failed rewarding for future manual inspection.
    ///
    /// Currently there is no automated recovery mechanism.
    ///
    /// # Arguments
    ///
    /// * `failure_data`: information regarding nodes that might have not received reward this interval.
    ///
    /// * `interval_rewarding_id`: id of the current interval rewarding.
    async fn save_failure_information(
        &self,
        failure_data: FailureData,
        interval_rewarding_id: i64,
    ) -> Result<(), RewardingError> {
        if let Some(failed_mixnode_chunks) = failure_data.mixnodes {
            for failed_chunk in failed_mixnode_chunks.into_iter() {
                // save the chunk
                let chunk_id = self
                    .storage
                    .insert_failed_mixnode_reward_chunk(FailedMixnodeRewardChunk {
                        interval_rewarding_id,
                        error_message: failed_chunk.error_message,
                    })
                    .await?;

                // and then all associated nodes
                for node in failed_chunk.possibly_unrewarded.into_iter() {
                    self.storage
                        .insert_possibly_unrewarded_mixnode(PossiblyUnrewardedMixnode {
                            chunk_id,
                            identity: node.identity,
                            uptime: node.params.uptime() as u8,
                        })
                        .await?;
                }
            }
        }

        Ok(())
    }

    /// Determines whether this validator has already distributed rewards for the specified interval
    /// so that it wouldn't accidentally attempt to do it again.
    ///
    /// # Arguments
    ///
    /// * `interval`: interval to check
    async fn check_if_rewarding_happened_at_interval(
        &self,
        interval: Interval,
    ) -> Result<bool, RewardingError> {
        if let Some(entry) = self
            .storage
            .get_interval_rewarding_entry(interval.start_unix_timestamp())
            .await?
        {
            // log error if the attempt wasn't finished. This error implies the process has crashed
            // during the rewards distribution
            if !entry.finished {
                error!(
                    "It seems that we haven't successfully finished distributing rewards at {}",
                    interval
                )
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Determines whether the specified interval is eligible for rewards, i.e. it was not rewarded
    /// before and we have enough network monitor test data to distribute the rewards based on them.
    ///
    /// # Arguments
    ///
    /// * `interval`: interval to check
    async fn check_interval_eligibility(&self, interval: Interval) -> Result<bool, RewardingError> {
        if self
            .check_if_rewarding_happened_at_interval(interval)
            .await?
            || !self.check_for_monitor_data(interval).await?
        {
            Ok(false)
        } else {
            // we haven't sent rewards during the interval and we have enough monitor test data
            Ok(true)
        }
    }

    /// Distribute rewards to all eligible mixnodes and gateways on the network.
    ///
    /// # Arguments
    ///
    /// * `interval`: current rewarding interval
    async fn perform_rewarding(&self, interval: Interval) -> Result<(), RewardingError> {
        info!(
            "Starting mixnode and gateway rewarding for interval {} ...",
            interval
        );

        // insert information about beginning the procedure (so that if we crash during it,
        // we wouldn't attempt to possibly double reward operators)
        let interval_rewarding_id = self
            .storage
            .insert_started_interval_rewarding(
                interval.start_unix_timestamp(),
                interval.end_unix_timestamp(),
            )
            .await?;

        let (report, failure_data) = self
            .distribute_rewards(interval_rewarding_id, interval)
            .await?;

        self.storage
            .finish_rewarding_interval_and_insert_report(report)
            .await?;

        if let Some(failure_data) = failure_data {
            if let Err(err) = self
                .save_failure_information(failure_data, interval_rewarding_id)
                .await
            {
                error!("failed to save information about rewarding failures!");
                // TODO: should we just terminate the process here?
                return Err(err);
            }
        }

        // since we have already performed rewards, purge everything older than the end of this interval
        // (+one day of buffer) as we're never going to need it again (famous last words...)
        // note that usually end of interval is equal to the current time
        let cutoff = (interval.end() - ONE_DAY).unix_timestamp();
        self.storage.purge_old_statuses(cutoff).await?;

        Ok(())
    }

    /// Checks whether there is enough network monitor test run data to distribute rewards
    /// for the specified interval.
    ///
    /// # Arguments
    ///
    /// * `interval`: interval to check
    async fn check_for_monitor_data(&self, interval: Interval) -> Result<bool, RewardingError> {
        let since = interval.start_unix_timestamp();
        let until = interval.end_unix_timestamp();

        let monitor_runs = self.storage.get_monitor_runs_count(since, until).await?;

        // check if we have more than threshold percentage of monitor runs for the interval
        let available = monitor_runs as f32 * 100.0 / self.expected_interval_monitor_runs as f32;
        Ok(available >= self.minimum_interval_monitor_threshold as f32)
    }

    async fn sync_up_rewarding_intervals(&self) -> Result<(), RewardingError> {
        let mut last_stored_interval = self.nymd_client.get_current_interval().await?;

        let block_now: OffsetDateTime = self.nymd_client.current_block_timestamp().await?.into();
        let actual_current_interval = match last_stored_interval.current(block_now) {
            None => return Ok(()),
            Some(interval) => interval,
        };

        // we're waiting for the first interval to start... (same is true if the value was 'None')
        if actual_current_interval.start() < last_stored_interval.start() {
            return Ok(());
        }

        // we're already synced up
        if actual_current_interval == last_stored_interval {
            return Ok(());
        }

        // actual_current_interval > last_stored_interval
        loop {
            // if we can perform rewarding, do it, otherwise just go straight into the next interval
            if self
                .check_interval_eligibility(last_stored_interval)
                .await?
            {
                self.perform_rewarding(last_stored_interval).await?;
            } else {
                self.nymd_client.advance_current_interval().await?;
            }

            last_stored_interval = self.nymd_client.get_current_interval().await?;

            // compare by start times in case the id didn't match (TODO: is it even possible?)
            if last_stored_interval.start() == actual_current_interval.start() {
                break;
            }
        }

        Ok(())
    }

    // pub(crate) async fn processing_loop_iteration(&self) -> Result<std::ops::ControlFlow<()>, RewardingError> {
    pub(crate) async fn processing_loop_iteration(&self) -> Result<(), RewardingError> {
        let last_stored_interval = self.nymd_client.get_current_interval().await?;
        let block_now: OffsetDateTime = self.nymd_client.current_block_timestamp().await?.into();

        let actual_current_interval = match last_stored_interval.current(block_now) {
            None => return Ok(()),
            Some(interval) => interval,
        };

        // the [stored] interval has finished - we should distribute rewards now
        if last_stored_interval.start() < actual_current_interval.start() {
            // it's time to distribute rewards, however, first let's see if we have enough data to go through with it
            // (consider the case of rewards being distributed every 24h at 12:00pm and validator-api
            // starting for the very first time at 11:00am. It's not going to have enough data for
            // rewards for the *current* interval, but we couldn't have known that at startup)
            if self
                .check_interval_eligibility(last_stored_interval)
                .await?
            {
                self.perform_rewarding(last_stored_interval).await?;
            } else {
                warn!("We do not have sufficient monitor data to perform rewarding in this interval ({}). We're advancing it forward...", last_stored_interval);
                self.nymd_client.advance_current_interval().await?;
            }
        } else {
            info!(
                "rewards will be distributed around {}. Approximately {:?} remaining",
                last_stored_interval.end(),
                last_stored_interval.until_end(block_now)
            );
        }

        sleep(Duration::from_secs(15 * 60)).await;

        Ok(())
    }

    pub(crate) async fn run(&self) {
        // whatever happens, we shouldn't do anything until the cache is initialised
        self.validator_cache.wait_for_initial_values().await;

        if let Err(err) = self.sync_up_rewarding_intervals().await {
            error!(
                "Failed to sync up intervals with the contract state - {}",
                err
            );
            process::exit(1);
        }

        // at this point the current block time < end of current[or first] interval, so to do anything,
        // we have to wait for the interval to finish
        loop {
            if let Err(err) = self.processing_loop_iteration().await {
                error!("failed to finish rewarding loop iteration - {}", err);
                // should we go into backoff here or just exit the process?
                sleep(Duration::from_secs(15 * 60)).await;
            }
        }
    }
}
