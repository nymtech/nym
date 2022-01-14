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
use config::defaults::DENOM;
use log::{error, info};
use mixnet_contract_common::mixnode::NodeRewardParams;
use mixnet_contract_common::{
    Epoch, ExecuteMsg, IdentityKey, MixNodeBond, RewardingStatus, MIXNODE_DELEGATORS_PAGE_LIMIT,
};
use std::collections::HashSet;
use std::convert::TryInto;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::sleep;
use validator_client::nymd::SigningNymdClient;

pub(crate) mod error;

#[derive(Copy, Clone)]
pub(crate) struct EpochRewardParams {
    pub(crate) reward_pool: u128,
    pub(crate) circulating_supply: u128,
    pub(crate) sybil_resistance_percent: u8,
    pub(crate) rewarded_set_size: u32,
    pub(crate) active_set_size: u32,
    pub(crate) period_reward_pool: u128,
    pub(crate) active_set_work_factor: u8,
}

impl EpochRewardParams {
    // technically it's identical to what would have been derived with a Default implementation,
    // however, I prefer to be explicit about it, as a `Default::default` value makes no sense
    // apart from the `ValidatorCacheInner` context, where this value is not going to be touched anyway
    // (it's guarded behind an `initialised` flag)
    pub(crate) fn new_empty() -> Self {
        EpochRewardParams {
            reward_pool: 0,
            circulating_supply: 0,
            sybil_resistance_percent: 0,
            rewarded_set_size: 0,
            active_set_size: 0,
            period_reward_pool: 0,
            active_set_work_factor: 0,
        }
    }

    pub(crate) fn estimate_reward(
        &self,
        node: &MixNodeBond,
        uptime: u8,
        in_active_set: bool,
    ) -> (u128, u128, u128) {
        let node_reward_params = NodeRewardParams::new(
            self.period_reward_pool,
            self.rewarded_set_size.into(),
            self.active_set_size.into(),
            0,
            self.circulating_supply,
            uptime.into(),
            self.sybil_resistance_percent,
            in_active_set,
            self.active_set_work_factor,
        );

        let total_node_reward = node.reward(&node_reward_params);
        let operator_reward = node.operator_reward(&node_reward_params);
        let delegators_reward =
            node.reward_delegation(node.total_delegation().amount, &node_reward_params);

        (
            total_node_reward
                .reward()
                .checked_to_num()
                .unwrap_or_default(),
            operator_reward,
            delegators_reward,
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MixnodeToReward {
    pub(crate) identity: IdentityKey,

    /// Total number of individual addresses that have delegated to this particular node
    pub(crate) total_delegations: usize,

    /// Node absolute uptime over total active set uptime
    pub(crate) params: NodeRewardParams,
}

impl MixnodeToReward {
    fn params(&self) -> NodeRewardParams {
        self.params
    }
}

impl MixnodeToReward {
    pub(crate) fn to_reward_execute_msg(&self, epoch_id: u32) -> ExecuteMsg {
        ExecuteMsg::RewardMixnode {
            identity: self.identity.clone(),
            params: self.params(),
            epoch_id,
        }
    }

    pub(crate) fn to_next_delegator_reward_execute_msg(&self, epoch_id: u32) -> ExecuteMsg {
        ExecuteMsg::RewardNextMixDelegators {
            mix_identity: self.identity.clone(),
            epoch_id,
        }
    }
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

    /// Current epoch
    current_epoch: Epoch,

    /// Ideal world, expected number of network monitor test runs per epoch.
    /// In reality it will be slightly lower due to network delays, but it's good enough
    /// for estimations regarding percentage of available data for reward distribution.
    expected_epoch_monitor_runs: usize,

    /// Minimum percentage of network monitor test runs reports required in order to distribute
    /// rewards.
    minimum_epoch_monitor_threshold: u8,
}

impl Rewarder {
    pub(crate) fn new(
        nymd_client: Client<SigningNymdClient>,
        validator_cache: ValidatorCache,
        storage: ValidatorApiStorage,
        current_epoch: Epoch,
        expected_epoch_monitor_runs: usize,
        minimum_epoch_monitor_threshold: u8,
    ) -> Self {
        Rewarder {
            nymd_client,
            validator_cache,
            storage,
            current_epoch,
            expected_epoch_monitor_runs,
            minimum_epoch_monitor_threshold,
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

    /// Obtain the list of current 'rewarded' set, determine their uptime in the provided epoch
    /// and attach information required for rewarding.
    ///
    /// The method also obtains the number of delegators towards the node in order to more accurately
    /// approximate the required gas fees when distributing the rewards.
    ///
    /// # Arguments
    ///
    /// * `epoch`: current rewarding epoch
    async fn determine_eligible_mixnodes(
        &self,
        epoch: &Epoch,
    ) -> Result<Vec<MixnodeToReward>, RewardingError> {
        let epoch_reward_params = self.nymd_client.get_current_epoch_reward_params().await?;

        info!("Rewarding pool stats");
        info!(
            "-- Reward pool: {} {}",
            epoch_reward_params.reward_pool, DENOM
        );
        info!(
            "---- Epoch reward pool: {} {}",
            epoch_reward_params.period_reward_pool, DENOM
        );
        info!(
            "-- Circulating supply: {} {}",
            epoch_reward_params.circulating_supply, DENOM
        );

        // 1. get list of all currently bonded nodes
        // 2. for each of them determine their delegator count
        // 3. for each of them determine their uptime for the epoch
        // 4. for each of them determine if they're currently in the active set
        // TODO: step 4 will definitely need to change.
        let all_nodes = self.validator_cache.mixnodes().await.into_inner();
        let active_set = self
            .validator_cache
            .active_set()
            .await
            .into_inner()
            .into_iter()
            .map(|bond| bond.mix_node.identity_key)
            .collect::<HashSet<_>>();

        let mut nodes_with_delegations = Vec::with_capacity(all_nodes.len());
        for node in all_nodes {
            let delegator_count = self
                .get_mixnode_delegators_count(node.mix_node.identity_key.clone())
                .await?;
            nodes_with_delegations.push((node, delegator_count));
        }

        let mut eligible_nodes = Vec::with_capacity(nodes_with_delegations.len());
        for (rewarded_node, total_delegations) in nodes_with_delegations.into_iter() {
            let uptime = self
                .storage
                .get_average_mixnode_uptime_in_interval(
                    rewarded_node.identity(),
                    epoch.start_unix_timestamp(),
                    epoch.end_unix_timestamp(),
                )
                .await?;

            eligible_nodes.push(MixnodeToReward {
                identity: rewarded_node.identity().clone(),
                total_delegations,
                params: NodeRewardParams::new(
                    epoch_reward_params.period_reward_pool,
                    epoch_reward_params.rewarded_set_size.into(),
                    epoch_reward_params.active_set_size.into(),
                    // Reward blockstamp gets set in the contract call
                    0,
                    epoch_reward_params.circulating_supply,
                    uptime.u8().into(),
                    epoch_reward_params.sybil_resistance_percent,
                    active_set.contains(rewarded_node.identity()),
                    epoch_reward_params.active_set_work_factor,
                ),
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
    /// * `epoch_id`: nonce associated with the current rewarding interval
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
    /// * `epoch_id`: nonce associated with the current rewarding interval.
    /// * `retry`: flag to indicate whether this is a retry attempt for rewarding particular nodes.
    async fn distribute_rewards_to_mixnodes(
        &self,
        eligible_mixnodes: &[MixnodeToReward],
        epoch_id: u32,
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
        let mut individually_rewarded = Vec::new();

        // sets of nodes that together they have < MIXNODE_DELEGATORS_PAGE_LIMIT delegators
        let mut batch_rewarded = vec![vec![]];
        let mut current_batch_i = 0;
        let mut current_batch_total = 0;

        // right now put mixes into batches super naively, if it doesn't fit into the current one,
        // create a new one.
        for mix in eligible_mixnodes {
            // if mixnode has uptime of 0, no rewarding will actually happen regardless of number of delegators,
            // so we can just batch it with the current batch
            if mix.params.uptime() == 0 {
                batch_rewarded[current_batch_i].push(mix.clone());
                continue;
            }

            if mix.total_delegations > MIXNODE_DELEGATORS_PAGE_LIMIT {
                individually_rewarded.push(mix)
            } else if current_batch_total + mix.total_delegations < MIXNODE_DELEGATORS_PAGE_LIMIT {
                batch_rewarded[current_batch_i].push(mix.clone());
                current_batch_total += mix.total_delegations;
            } else {
                batch_rewarded.push(vec![mix.clone()]);
                current_batch_i += 1;
                current_batch_total = 0;
            }
        }

        let mut total_rewarded = 0;

        // start rewarding, first the nodes that are dealt with individually, i.e. nodes that
        // need to have their own special blocks due to number of delegators
        for mix in individually_rewarded {
            if let Err(err) = self
                .nymd_client
                .reward_mixnode_and_all_delegators(mix, epoch_id)
                .await
            {
                // this is a super weird edge case that we didn't catch change to sequence and
                // resent rewards unnecessarily, but the mempool saved us from executing it again
                // however, still we want to wait until we're sure we're into the next block
                if !err.is_tendermint_duplicate() {
                    error!("failed to reward mixnode with all delegators... - {}", err);
                    failed_chunks.push(FailedMixnodeRewardChunkDetails {
                        possibly_unrewarded: vec![mix.clone()],
                        error_message: err.to_string(),
                    });
                }
                sleep(Duration::from_secs(11)).await;
            }

            total_rewarded += 1;
            self.print_rewarding_progress(total_rewarded, eligible_mixnodes.len(), retry);
        }

        // then we move onto the chunks
        for mix_chunk in batch_rewarded {
            if mix_chunk.is_empty() {
                continue;
            }
            if let Err(err) = self
                .nymd_client
                .reward_mixnodes_with_single_page_of_delegators(&mix_chunk, epoch_id)
                .await
            {
                // this is a super weird edge case that we didn't catch change to sequence and
                // resent rewards unnecessarily, but the mempool saved us from executing it again
                // however, still we want to wait until we're sure we're into the next block
                if !err.is_tendermint_duplicate() {
                    error!("failed to reward mixnodes... - {}", err);
                    failed_chunks.push(FailedMixnodeRewardChunkDetails {
                        possibly_unrewarded: mix_chunk.to_vec(),
                        error_message: err.to_string(),
                    });
                }
                sleep(Duration::from_secs(11)).await;
            }

            total_rewarded += mix_chunk.len();
            self.print_rewarding_progress(total_rewarded, eligible_mixnodes.len(), retry);
        }

        if failed_chunks.is_empty() {
            None
        } else {
            Some(failed_chunks)
        }
    }

    /// For each mixnode on the list, try to "continue" rewarding its delegators.
    /// Note: due to the checks inside the smart contract, it's impossible to accidentally
    /// reward the same mixnode (or delegator) twice during particular rewarding interval.
    ///
    /// Realistically if this method is ever called, it will be only done once per node, so there's
    /// no need to determine the exact number of missed delegators.
    ///
    /// * `nodes`: mixnodes which delegators did not receive all rewards in this epoch.
    /// * `epoch_id`: nonce associated with the current rewarding interval.
    async fn reward_missed_delegators(&self, nodes: &[MixnodeToReward], epoch_id: u32) {
        let mut total_resent = 0;
        for missed_node in nodes {
            total_resent += 1;
            info!(
                "Sending rewarding transaction for missed delegators ({} / {} mixnodes re-checked)",
                total_resent,
                nodes.len()
            );

            if let Err(err) = self
                .nymd_client
                .reward_mix_delegators(missed_node, epoch_id)
                .await
            {
                warn!(
                    "failed to attempt to reward missed delegators of node {} - {}",
                    missed_node.identity, err
                )
            }
        }
    }

    /// Using the list of active mixnode and gateways, determine which of them are eligible for
    /// rewarding and distribute the rewards.
    ///
    /// # Arguments
    ///
    /// * `epoch_rewarding_id`: id of the current epoch rewarding as stored in the database.
    /// * `epoch`: current rewarding epoch
    async fn distribute_rewards(
        &self,
        epoch_rewarding_database_id: i64,
        epoch: &Epoch,
    ) -> Result<(RewardingReport, Option<FailureData>), RewardingError> {
        let mut failure_data = FailureData::default();

        let eligible_mixnodes = self.determine_eligible_mixnodes(epoch).await?;
        if eligible_mixnodes.is_empty() {
            return Err(RewardingError::NoMixnodesToReward);
        }
        let total_eligible = eligible_mixnodes.len();

        failure_data.mixnodes = self
            .distribute_rewards_to_mixnodes(&eligible_mixnodes, epoch.id(), false)
            .await;

        let mut nodes_to_verify = eligible_mixnodes;

        // if there's some underlying networking error or something, don't keep retrying forever
        let mut retries_allowed = 5;
        loop {
            if retries_allowed <= 0 {
                break;
            }
            let (unrewarded, mut pending_delegators) = self
                .verify_rewarding_completion(&nodes_to_verify, epoch.id())
                .await;
            if unrewarded.is_empty() && pending_delegators.is_empty() {
                // we're all good - everyone got their rewards
                break;
            }

            if !unrewarded.is_empty() {
                // no need to save failure data as we already know about those from the very first run
                self.distribute_rewards_to_mixnodes(&unrewarded, epoch.id(), true)
                    .await;
            }

            if !pending_delegators.is_empty() {
                self.reward_missed_delegators(&pending_delegators, epoch.id())
                    .await;
            }

            // no point in verifying EVERYTHING again, just check the nodes that went through retries
            nodes_to_verify = unrewarded;
            nodes_to_verify.append(&mut pending_delegators);

            retries_allowed -= 1;
        }

        let report = RewardingReport {
            epoch_rewarding_id: epoch_rewarding_database_id,
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

        self.nymd_client.advance_current_epoch().await?;

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
    /// * `failure_data`: information regarding nodes that might have not received reward this epoch.
    ///
    /// * `epoch_rewarding_id`: id of the current epoch rewarding.
    async fn save_failure_information(
        &self,
        failure_data: FailureData,
        epoch_rewarding_id: i64,
    ) -> Result<(), RewardingError> {
        if let Some(failed_mixnode_chunks) = failure_data.mixnodes {
            for failed_chunk in failed_mixnode_chunks.into_iter() {
                // save the chunk
                let chunk_id = self
                    .storage
                    .insert_failed_mixnode_reward_chunk(FailedMixnodeRewardChunk {
                        epoch_rewarding_id,
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

    /// Determines whether this validator has already distributed rewards for the specified epoch
    /// so that it wouldn't accidentally attempt to do it again.
    ///
    /// # Arguments
    ///
    /// * `epoch`: epoch to check
    async fn check_if_rewarding_happened_at_epoch(
        &self,
        epoch: &Epoch,
    ) -> Result<bool, RewardingError> {
        if let Some(entry) = self
            .storage
            .get_epoch_rewarding_entry(epoch.start_unix_timestamp())
            .await?
        {
            // log error if the attempt wasn't finished. This error implies the process has crashed
            // during the rewards distribution
            if !entry.finished {
                error!(
                    "It seems that we haven't successfully finished distributing rewards at {}",
                    epoch
                )
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Determines whether the specified epoch is eligible for rewards, i.e. it was not rewarded
    /// before and we have enough network monitor test data to distribute the rewards based on them.
    ///
    /// # Arguments
    ///
    /// * `epoch`: epoch to check
    async fn check_epoch_eligibility(&self, epoch: &Epoch) -> Result<bool, RewardingError> {
        if self.check_if_rewarding_happened_at_epoch(epoch).await?
            || !self.check_for_monitor_data(epoch).await?
        {
            Ok(false)
        } else {
            // we haven't sent rewards during the epoch and we have enough monitor test data
            Ok(true)
        }
    }

    /// Determines the next epoch during which the rewards should get distributed.
    ///
    /// # Arguments
    ///
    /// * `now`: current datetime
    async fn next_rewarding_epoch(&self, now: OffsetDateTime) -> Result<Epoch, RewardingError> {
        // edge case handling for when we decide to change first epoch to be at some time in the future
        // (i.e. epoch length transition)
        // we don't have to perform checks here as it's impossible to distribute rewards for epochs
        // in the future
        if self.current_epoch.start() > now {
            return Ok(self.current_epoch.clone());
        }

        let current_epoch = self.current_epoch.current(now);
        // check previous epoch in case we had a tiny hiccup
        // example:
        // epochs start at 12:00pm and last for 24h (ignore variance)
        // validator-api crashed at 11:59am before distributing rewards
        // and restarted at 12:01 - it has all the data required to distribute the rewards
        // for the previous epoch.
        let previous_epoch = current_epoch.previous_epoch();
        if self.check_epoch_eligibility(&previous_epoch).await? {
            return Ok(previous_epoch);
        }

        // check if rewards weren't already given out for the current epoch
        // (it can happen for negative variance if the process crashed)
        // note that if the epoch ends at say 12:00 and it's 11:59 and we just started,
        // we might end up skipping this epoch regardless
        if !self
            .check_if_rewarding_happened_at_epoch(&current_epoch)
            .await?
        {
            return Ok(current_epoch);
        }

        // if we have given rewards for the previous and the current epoch,
        // wait until the next one
        Ok(current_epoch.next_epoch())
    }

    /// Given datetime of the rewarding epoch datetime, determine duration until it ends.
    ///
    /// # Arguments
    ///
    /// * `rewarding_epoch`: the rewarding epoch
    fn determine_delay_until_next_rewarding(&self, rewarding_epoch: &Epoch) -> Option<Duration> {
        let now = OffsetDateTime::now_utc();
        if now > rewarding_epoch.end() {
            return None;
        }

        // we have a positive duration so we can't fail the conversion
        let until_epoch_end: Duration = (rewarding_epoch.end() - now).try_into().unwrap();

        Some(until_epoch_end)
    }

    /// Distribute rewards to all eligible mixnodes and gateways on the network.
    ///
    /// # Arguments
    ///
    /// * `epoch`: current rewarding epoch
    async fn perform_rewarding(&self, epoch: Epoch) -> Result<(), RewardingError> {
        info!(
            "Starting mixnode and gateway rewarding for epoch {} ...",
            epoch
        );

        // insert information about beginning the procedure (so that if we crash during it,
        // we wouldn't attempt to possibly double reward operators)
        let epoch_rewarding_id = self
            .storage
            .insert_started_epoch_rewarding(epoch.start_unix_timestamp())
            .await?;

        let (report, failure_data) = self.distribute_rewards(epoch_rewarding_id, &epoch).await?;

        self.storage
            .finish_rewarding_epoch_and_insert_report(report)
            .await?;

        if let Some(failure_data) = failure_data {
            if let Err(err) = self
                .save_failure_information(failure_data, epoch_rewarding_id)
                .await
            {
                error!("failed to save information about rewarding failures!");
                // TODO: should we just terminate the process here?
                return Err(err);
            }
        }

        // since we have already performed rewards, purge everything older than the end of this epoch
        // (+one day of buffer) as we're never going to need it again (famous last words...)
        // note that usually end of epoch is equal to the current time
        let cutoff = (epoch.end() - ONE_DAY).unix_timestamp();
        self.storage.purge_old_statuses(cutoff).await?;

        Ok(())
    }

    /// Checks whether there is enough network monitor test run data to distribute rewards
    /// for the specified epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch`: epoch to check
    async fn check_for_monitor_data(&self, epoch: &Epoch) -> Result<bool, RewardingError> {
        let since = epoch.start_unix_timestamp();
        let until = epoch.end_unix_timestamp();

        let monitor_runs = self.storage.get_monitor_runs_count(since, until).await?;

        // check if we have more than threshold percentage of monitor runs for the epoch
        let available = monitor_runs as f32 * 100.0 / self.expected_epoch_monitor_runs as f32;
        Ok(available >= self.minimum_epoch_monitor_threshold as f32)
    }

    /// Waits until the next epoch starts
    ///
    /// # Arguments
    ///
    /// * `current_epoch`: current epoch that we want to wait out
    async fn wait_until_next_epoch(&self, current_epoch: Epoch) {
        let now = OffsetDateTime::now_utc();
        let until_end = current_epoch.end() - now;

        // otherwise it means the epoch is already over and the next one has begun
        if until_end.is_positive() {
            // we know for sure that the duration here is positive so conversion can't fail
            sleep(until_end.try_into().unwrap()).await;
        }
    }

    pub(crate) async fn run(&self) {
        // whatever happens, we shouldn't do anything until the cache is initialised
        self.validator_cache.wait_for_initial_values().await;

        loop {
            // Just a reference for anyone wanting to modify the code to use blockchain timestamps.
            // This method is now available:
            // let current_block_timestamp = self.nymd_client.current_block_timestamp().await.unwrap();
            // and if you look at the source of that, you can easily use block height instead if preferred.

            let now = OffsetDateTime::now_utc();
            // if we haven't rewarded anyone for the previous epoch, get the start of the previous epoch
            // otherwise get the start of the current epoch
            // (remember, we will be rewarding at the END of the selected epoch)
            let next_rewarding_epoch = match self.next_rewarding_epoch(now).await {
                Ok(next_rewarding_epoch) => next_rewarding_epoch,
                Err(err) => {
                    // I'm not entirely sure whether this is recoverable, because failure implies database errors
                    error!("We failed to determine time until next reward cycle ({}). Going to wait for the epoch length until next attempt", err);
                    sleep(self.current_epoch.length()).await;
                    continue;
                }
            };

            // wait's until the start of the *next* epoch, e.g. end of the current chosen epoch
            // (it could be none, for example if we are distributing overdue rewards for the previous epoch)
            if let Some(remaining_time) =
                self.determine_delay_until_next_rewarding(&next_rewarding_epoch)
            {
                info!("Next rewarding epoch is {}", next_rewarding_epoch);
                info!(
                    "Rewards distribution will happen at {}. ({:?} left)",
                    now + remaining_time,
                    remaining_time
                );
                sleep(remaining_time).await;
            } else {
                info!(
                    "Starting reward distribution for epoch {} immediately!",
                    next_rewarding_epoch
                );
            }

            // it's time to distribute rewards, however, first let's see if we have enough data to go through with it
            // (consider the case of rewards being distributed every 24h at 12:00pm and validator-api
            // starting for the very first time at 11:00am. It's not going to have enough data for
            // rewards for the *current* epoch, but we couldn't have known that at startup)
            match self.check_for_monitor_data(&next_rewarding_epoch).await {
                Err(_) | Ok(false) => {
                    warn!("We do not have sufficient monitor data to perform rewarding in this epoch ({})", next_rewarding_epoch);
                    self.wait_until_next_epoch(next_rewarding_epoch).await;
                    continue;
                }
                _ => (),
            }

            if let Err(err) = self.perform_rewarding(next_rewarding_epoch).await {
                // TODO: should we just terminate the process here instead?
                error!("Failed to distribute rewards! - {}", err)
            }
        }
    }
}
