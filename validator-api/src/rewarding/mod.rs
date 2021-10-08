// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cache::ValidatorCache;
use crate::node_status_api::models::{GatewayStatusReport, MixnodeStatusReport, Uptime};
use crate::node_status_api::ONE_DAY;
use crate::nymd_client::Client;
use crate::rewarding::epoch::Epoch;
use crate::rewarding::error::RewardingError;
use crate::storage::models::{
    FailedGatewayRewardChunk, FailedMixnodeRewardChunk, PossiblyUnrewardedGateway,
    PossiblyUnrewardedMixnode, RewardingReport,
};
use crate::storage::NodeStatusStorage;
use log::{error, info};
use mixnet_contract::{ExecuteMsg, IdentityKey};
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::sleep;
use validator_client::nymd::SigningNymdClient;

pub(crate) mod epoch;
pub(crate) mod error;

// the actual base cost is around 125_000, but let's give ourselves a bit of safety net in case
// we introduce some tiny contract changes that would bump that value up
pub(crate) const MIXNODE_REWARD_OP_BASE_GAS_LIMIT: u64 = 150_000;
pub(crate) const GATEWAY_REWARD_OP_BASE_GAS_LIMIT: u64 = 150_000;

// For each delegation reward we perform a read and a write is being executed,
// which are the most costly parts involved in process. Both of them are ~1000 sdk gas in cost.
// However, experimentally it looks like first delegation adds total of additional ~3000 of sdk gas
// cost and each subsequent about ~2500.
// Therefore, since base cost is not tuned to the bare minimum, let's treat all of delegations as extra
// 2750 of sdk gas.
pub(crate) const PER_MIXNODE_DELEGATION_GAS_INCREASE: u64 = 2750;
pub(crate) const PER_GATEWAY_DELEGATION_GAS_INCREASE: u64 = 2750;

// Another safety net in case of contract changes,
// the calculated total gas limit is going to get multiplied by that value.
pub(crate) const REWARDING_GAS_LIMIT_MULTIPLIER: f64 = 1.05;

pub(crate) const MAX_TO_REWARD_AT_ONCE: usize = 50;

pub(crate) const REWARDING_TIME_VARIANCE: f32 = 0.05; // 5% (so for example +/-1.2h for 24h epoch)

#[derive(Debug, Clone)]
pub(crate) struct MixnodeToReward {
    pub(crate) identity: IdentityKey,
    pub(crate) uptime: Uptime,

    /// Total number of individual addresses that have delegated to this particular node
    pub(crate) total_delegations: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct GatewayToReward {
    pub(crate) identity: IdentityKey,
    pub(crate) uptime: Uptime,

    /// Total number of individual addresses that have delegated to this particular gateway
    pub(crate) total_delegations: usize,
}

pub(crate) struct FailedMixnodeRewardChunkDetails {
    possibly_unrewarded: Vec<MixnodeToReward>,
    error_message: String,
}

pub(crate) struct FailedGatewayRewardChunkDetails {
    possibly_unrewarded: Vec<GatewayToReward>,
    error_message: String,
}

#[derive(Default)]
pub(crate) struct FailureData {
    mixnodes: Option<Vec<FailedMixnodeRewardChunkDetails>>,
    gateways: Option<Vec<FailedGatewayRewardChunkDetails>>,
}

impl<'a> From<&'a MixnodeToReward> for ExecuteMsg {
    fn from(node: &MixnodeToReward) -> Self {
        ExecuteMsg::RewardMixnode {
            identity: node.identity.clone(),
            uptime: node.uptime.u8() as u32,
        }
    }
}

impl<'a> From<&'a GatewayToReward> for ExecuteMsg {
    fn from(node: &GatewayToReward) -> Self {
        ExecuteMsg::RewardGateway {
            identity: node.identity.clone(),
            uptime: node.uptime.u8() as u32,
        }
    }
}

pub(crate) struct Rewarder {
    nymd_client: Client<SigningNymdClient>,
    validator_cache: ValidatorCache,
    storage: NodeStatusStorage,

    /// The first epoch of the current length.
    first_epoch: Epoch,

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
        storage: NodeStatusStorage,
        first_epoch: Epoch,
        expected_epoch_monitor_runs: usize,
        minimum_epoch_monitor_threshold: u8,
    ) -> Self {
        Rewarder {
            nymd_client,
            validator_cache,
            storage,
            first_epoch,
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

    /// Obtains the current number of delegators that have delegated their stake towards this particular gateway.
    ///
    /// # Arguments
    ///
    /// * `identity`: identity key of the gateway
    async fn get_gateway_delegators_count(
        &self,
        identity: IdentityKey,
    ) -> Result<usize, RewardingError> {
        Ok(self
            .nymd_client
            .get_gateway_delegations(identity)
            .await?
            .len())
    }

    /// Queries the smart contract in order to obtain the current list of bonded mixnodes and then
    /// for each mixnode determines how many delegators it has.
    async fn produce_active_mixnode_delegators_map(
        &self,
    ) -> Result<HashMap<IdentityKey, usize>, RewardingError> {
        // Technically we could optimise it by creating a concurrent stream and executing multiple
        // queries concurrently.
        //
        // I've actually tested that approach and for 5300 nodes running it all sequentially was taking around 19s
        // while running it with 20 concurrent queries was taking around 4.5s.
        // Note that the results were a bit biased as I was testing it against remote validator
        // while in real world this would be making only local requests.
        // During the test my average ping times to the machine were around 2.6ms.
        // So I guess the network latency was 2.6ms * 5300 = 13.78s in total in the sequential case.
        //
        // HOWEVER, even if the method was taking that long in real world,
        // in the grand scheme of things it makes absolutely no difference. If the rewards
        // distribution is delayed by 15s, it changes nothing as the process itself is not
        // instantaneous.
        let mut map = HashMap::new();

        let active_bonded_mixnodes = self
            .validator_cache
            .active_mixnodes()
            .await
            .ok_or(RewardingError::NoMixnodesToReward)?
            .into_inner();
        for mix in active_bonded_mixnodes.into_iter() {
            let delegator_count = self
                .get_mixnode_delegators_count(mix.mix_node.identity_key.clone())
                .await?;
            map.insert(mix.mix_node.identity_key, delegator_count);
        }

        Ok(map)
    }

    /// Queries the smart contract in order to obtain the current list of bonded gateways and then
    /// for each gateway determines how many delegators it has.
    async fn produce_active_gateway_delegators_map(
        &self,
    ) -> Result<HashMap<IdentityKey, usize>, RewardingError> {
        // look at comments in `produce_mixnode_delegators_map` for some optimisation elaboration
        let mut map = HashMap::new();

        let active_bonded_gateways = self
            .validator_cache
            .active_gateways()
            .await
            .ok_or(RewardingError::NoGatewaysToReward)?
            .into_inner();
        for gateway in active_bonded_gateways.into_iter() {
            let delegator_count = self
                .get_gateway_delegators_count(gateway.gateway.identity_key.clone())
                .await?;
            map.insert(gateway.gateway.identity_key, delegator_count);
        }

        Ok(map)
    }

    /// Calculates the absolute uptime of given node that is then passed as one of the arguments
    /// in the smart contract to determine the actual reward value.
    ///
    /// Currently both ipv4 and ipv6 uptimes carry the same weight in the calculation.
    ///
    /// # Arguments
    ///
    /// * `ipv4_uptime`: ipv4 uptime of the node in the last epoch.
    /// * `ipv6_uptime`: ipv6 uptime of the node in the last epoch.
    fn calculate_absolute_uptime(&self, ipv4_uptime: Uptime, ipv6_uptime: Uptime) -> Uptime {
        // just take average of ipv4 and ipv6 uptimes using equal weights
        let abs = ((ipv4_uptime.u8() as f32 + ipv6_uptime.u8() as f32) / 2.0).round();
        Uptime::try_from(abs as i64).unwrap()
    }

    /// Given the list of mixnodes that were tested in the last epoch, tries to determine the
    /// subset that are eligible for any rewards.
    ///
    /// As of right now, it is a rather straightforward process. It is checked whether the node
    /// is currently bonded, has uptime > 0 and is part of the "active" set.
    /// Unlike the typescript rewards script, it currently does not look at the verloc data nor
    /// whether the non-mixing ports are open.
    ///
    /// The method also obtains the number of delegators towards the node in order to more accurately
    /// approximate the required gas fees when distributing the rewards.
    ///
    /// # Arguments
    ///
    /// * `active_mixnodes`: list of the nodes that were tested at least once by the network monitor
    ///                      in the last epoch.
    async fn determine_eligible_mixnodes(
        &self,
        active_mixnodes: &[MixnodeStatusReport],
    ) -> Result<Vec<MixnodeToReward>, RewardingError> {
        // Currently we don't have as many 'features' as in the typescript reward script,
        // such as we don't check ports or verloc data anymore. However, that's fine as
        // it's a good price to pay for being able to move rewarding to rust
        // and the lack of port data / verloc data will eventually be balanced out anyway
        // by people hesitating to delegate to nodes without them and thus those nodes disappearing
        // from the active set (once introduced)
        let mixnode_delegators = self.produce_active_mixnode_delegators_map().await?;

        // 1. go through all active mixnodes
        // 2. filter out nodes that are currently not in the active set (as `mixnode_delegators` was obtained by
        //    querying the validator)
        // 3. determine uptime and attach delegators count
        let eligible_nodes = active_mixnodes
            .iter()
            .filter_map(|mix| {
                mixnode_delegators
                    .get(&mix.identity)
                    .map(|&total_delegations| MixnodeToReward {
                        identity: mix.identity.clone(),
                        uptime: self
                            .calculate_absolute_uptime(mix.last_day_ipv4, mix.last_day_ipv6),
                        total_delegations,
                    })
            })
            .filter(|node| node.uptime.u8() > 0)
            .collect();

        Ok(eligible_nodes)
    }

    /// Given the list of gateways that were tested in the last epoch, tries to determine the
    /// subset that are eligible for any rewards.
    ///
    /// As of right now, it is a rather straightforward process. It is checked whether the node
    /// is currently bonded, has uptime > 0 and is part of the "active" set.
    /// Unlike the typescript rewards script, it currently does not look at the non-mixing ports are open.
    ///
    /// The method also obtains the number of delegators towards the node in order to more accurately
    /// approximate the required gas fees when distributing the rewards.
    ///
    /// # Arguments
    ///
    /// * `active_gateways`: list of the nodes that were tested at least once by the network monitor
    ///                      in the last epoch.
    async fn determine_eligible_gateways(
        &self,
        active_gateways: &[GatewayStatusReport],
    ) -> Result<Vec<GatewayToReward>, RewardingError> {
        let gateway_delegators = self.produce_active_gateway_delegators_map().await?;

        let eligible_nodes = active_gateways
            .iter()
            .filter_map(|gateway| {
                gateway_delegators
                    .get(&gateway.identity)
                    .map(|&total_delegations| GatewayToReward {
                        identity: gateway.identity.clone(),
                        uptime: self.calculate_absolute_uptime(
                            gateway.last_day_ipv4,
                            gateway.last_day_ipv6,
                        ),
                        total_delegations,
                    })
            })
            .filter(|node| node.uptime.u8() > 0)
            .collect();

        Ok(eligible_nodes)
    }

    /// Obtains the lists of all mixnodes and gateways that were tested at least a single time
    /// by the network monitor in the specified epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch`: the specified epoch.
    async fn get_active_monitor_nodes(
        &self,
        epoch: Epoch,
    ) -> Result<(Vec<MixnodeStatusReport>, Vec<GatewayStatusReport>), RewardingError> {
        let active_mixnodes = self
            .storage
            .get_all_active_mixnode_reports_in_interval(
                epoch.start_unix_timestamp(),
                epoch.end_unix_timestamp(),
            )
            .await?;

        let active_gateways = self
            .storage
            .get_all_active_gateway_reports_in_interval(
                epoch.start_unix_timestamp(),
                epoch.end_unix_timestamp(),
            )
            .await?;

        Ok((active_mixnodes, active_gateways))
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
    /// * `eligible_mixnodes`: list of the nodes that are eligible to receive non-zero rewards.
    async fn distribute_rewards_to_mixnodes(
        &self,
        eligible_mixnodes: &[MixnodeToReward],
    ) -> Option<Vec<FailedMixnodeRewardChunkDetails>> {
        let mut failed_chunks = Vec::new();

        for (i, mix_chunk) in eligible_mixnodes.chunks(MAX_TO_REWARD_AT_ONCE).enumerate() {
            if let Err(err) = self.nymd_client.reward_mixnodes(mix_chunk).await {
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
            let rewarded = i * MAX_TO_REWARD_AT_ONCE + mix_chunk.len();
            let percentage = rewarded as f32 * 100.0 / eligible_mixnodes.len() as f32;
            info!(
                "Rewarded {} / {} mixnodes\t{:.2}%",
                rewarded,
                eligible_mixnodes.len(),
                percentage
            );
        }

        if failed_chunks.is_empty() {
            None
        } else {
            Some(failed_chunks)
        }
    }

    /// Using the list of gateways eligible for rewards, chunks it into pre-defined sized-chunks
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
    /// * `eligible_gateways`: list of the nodes that are eligible to receive non-zero rewards.
    async fn distribute_rewards_to_gateways(
        &self,
        eligible_gateways: &[GatewayToReward],
    ) -> Option<Vec<FailedGatewayRewardChunkDetails>> {
        let mut failed_chunks = Vec::new();

        for (i, gateway_chunk) in eligible_gateways.chunks(MAX_TO_REWARD_AT_ONCE).enumerate() {
            if let Err(err) = self.nymd_client.reward_gateways(gateway_chunk).await {
                // this is a super weird edge case that we didn't catch change to sequence and
                // resent rewards unnecessarily, but the mempool saved us from executing it again
                // however, still we want to wait until we're sure we're into the next block
                if !err.is_tendermint_duplicate() {
                    error!("failed to reward gateways... - {}", err);
                    failed_chunks.push(FailedGatewayRewardChunkDetails {
                        possibly_unrewarded: gateway_chunk.to_vec(),
                        error_message: err.to_string(),
                    });
                }
                sleep(Duration::from_secs(11)).await;
            }

            let rewarded = i * MAX_TO_REWARD_AT_ONCE + gateway_chunk.len();
            let percentage = rewarded as f32 * 100.0 / eligible_gateways.len() as f32;
            info!(
                "Rewarded {} / {} gateways\t{:.2}%",
                rewarded,
                eligible_gateways.len(),
                percentage
            );
        }

        if failed_chunks.is_empty() {
            None
        } else {
            Some(failed_chunks)
        }
    }

    /// Using the list of active mixnode and gateways, determine which of them are eligible for
    /// rewarding and distribute the rewards.
    ///
    /// # Arguments
    ///
    /// * `epoch_rewarding_id`: id of the current epoch rewarding.
    ///
    /// * `active_monitor_mixnodes`: list of the nodes that were tested at least once by the network monitor
    ///                              in the last epoch.
    ///
    /// * `active_monitor_gateways`: list of the nodes that were tested at least once by the network monitor
    ///                              in the last epoch.
    async fn distribute_rewards(
        &self,
        epoch_rewarding_id: i64,
        active_monitor_mixnodes: &[MixnodeStatusReport],
        active_monitor_gateways: &[GatewayStatusReport],
    ) -> Result<(RewardingReport, Option<FailureData>), RewardingError> {
        let mut failure_data = FailureData::default();

        let eligible_mixnodes = self
            .determine_eligible_mixnodes(active_monitor_mixnodes)
            .await?;
        if eligible_mixnodes.is_empty() {
            return Err(RewardingError::NoMixnodesToReward);
        }

        let eligible_gateways = self
            .determine_eligible_gateways(active_monitor_gateways)
            .await?;
        if eligible_gateways.is_empty() {
            return Err(RewardingError::NoGatewaysToReward);
        }

        failure_data.mixnodes = self
            .distribute_rewards_to_mixnodes(&eligible_mixnodes)
            .await;

        failure_data.gateways = self
            .distribute_rewards_to_gateways(&eligible_gateways)
            .await;

        let report = RewardingReport {
            epoch_rewarding_id,
            eligible_mixnodes: eligible_mixnodes.len() as i64,
            eligible_gateways: eligible_gateways.len() as i64,
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
            possibly_unrewarded_gateways: failure_data
                .gateways
                .as_ref()
                .map(|chunks| {
                    chunks
                        .iter()
                        .map(|chunk| chunk.possibly_unrewarded.len())
                        .sum::<usize>() as i64
                })
                .unwrap_or_default(),
        };

        if failure_data.mixnodes.is_none() && failure_data.gateways.is_none() {
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
                            uptime: node.uptime.u8(),
                        })
                        .await?;
                }
            }
        }

        if let Some(failed_gateway_chunks) = failure_data.gateways {
            for failed_chunk in failed_gateway_chunks.into_iter() {
                // save the chunk
                let chunk_id = self
                    .storage
                    .insert_failed_gateway_reward_chunk(FailedGatewayRewardChunk {
                        epoch_rewarding_id,
                        error_message: failed_chunk.error_message,
                    })
                    .await?;

                // and then all associated nodes
                for node in failed_chunk.possibly_unrewarded.into_iter() {
                    self.storage
                        .insert_possibly_unrewarded_gateway(PossiblyUnrewardedGateway {
                            chunk_id,
                            identity: node.identity,
                            uptime: node.uptime.u8(),
                        })
                        .await?;
                }
            }
        }

        Ok(())
    }

    /// Determines random positive or negative time variance that should be added to the rewarding
    /// distribution time so that all validators would not attempt to hit the smart contract
    /// at exactly the same time.
    fn epoch_variance(&self) -> (bool, Duration) {
        let mut rng = thread_rng();

        let abs_variance_secs = REWARDING_TIME_VARIANCE * self.first_epoch.length().as_secs_f32();
        let variance = Duration::from_secs(rng.gen_range(0, abs_variance_secs as u64));
        let sign = rng.gen_bool(0.5);

        (sign, variance)
    }

    /// Determines whether this validator has already distributed rewards for the specified epoch
    /// so that it wouldn't accidentally attempt to do it again.
    ///
    /// # Arguments
    ///
    /// * `epoch`: epoch to check
    async fn check_if_rewarding_happened_at_epoch(
        &self,
        epoch: Epoch,
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
    async fn check_epoch_eligibility(&self, epoch: Epoch) -> Result<bool, RewardingError> {
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
        if self.first_epoch.start() > now {
            return Ok(self.first_epoch);
        }

        let current_epoch = self.first_epoch.current(now);
        // check previous epoch in case we had a tiny hiccup
        // example:
        // epochs start at 12:00pm and last for 24h (ignore variance)
        // validator-api crashed at 11:59am before distributing rewards
        // and restarted at 12:01 - it has all the data required to distribute the rewards
        // for the previous epoch.
        let previous_epoch = current_epoch.previous_epoch();
        if self.check_epoch_eligibility(previous_epoch).await? {
            return Ok(previous_epoch);
        }

        // check if rewards weren't already given out for the current epoch
        // (it can happen for negative variance if the process crashed)
        // note that if the epoch ends at say 12:00 and it's 11:59 and we just started,
        // we might end up skipping this epoch regardless
        if !self
            .check_if_rewarding_happened_at_epoch(current_epoch)
            .await?
        {
            return Ok(current_epoch);
        }

        // if we have given rewards for the previous and the current epoch,
        // wait until the next one
        Ok(current_epoch.next_epoch())
    }

    /// Given datetime of the rewarding epoch datetime, determine time until it ends and add (or remove)
    /// a little bit of time variance from it in order to prevent all validators distributing
    /// the rewards at exactly the same time instant.
    ///
    /// # Arguments
    ///
    /// * `rewarding_epoch`: the rewarding epoch
    fn determine_delay_until_next_rewarding(&self, rewarding_epoch: Epoch) -> Option<Duration> {
        let now = OffsetDateTime::now_utc();
        if now > rewarding_epoch.end() {
            return None;
        }

        // we have a positive duration so we can't fail the conversion
        let until_epoch_end: Duration = (rewarding_epoch.end() - now).try_into().unwrap();

        // add a bit of variance to the start time
        let (sign, variance) = self.epoch_variance();
        if sign {
            Some(until_epoch_end + variance)
        } else {
            until_epoch_end.checked_sub(variance)
        }
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

        // get nodes that were active during the epoch
        let (active_monitor_mixnodes, active_monitor_gateways) =
            self.get_active_monitor_nodes(epoch).await?;

        // insert information about beginning the procedure (so that if we crash during it,
        // we wouldn't attempt to possibly double reward operators)
        let epoch_rewarding_id = self
            .storage
            .insert_started_epoch_rewarding(epoch.start_unix_timestamp())
            .await?;

        let (report, failure_data) = self
            .distribute_rewards(
                epoch_rewarding_id,
                &active_monitor_mixnodes,
                &active_monitor_gateways,
            )
            .await?;

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

        // TODO: again, this assumes 24h epochs.
        let epoch_iso_8601 = epoch.start().date().to_string();
        let two_days_ago = (epoch.start() - 2 * ONE_DAY).unix_timestamp();

        // NOTE: this works under assumption that epochs are 24h in length.
        // If this changes then the historical uptime updates should be performed
        // on a timer in another task
        if self
            .storage
            .check_if_historical_uptimes_exist_for_date(&epoch_iso_8601)
            .await?
        {
            error!("We have already updated uptimes for all nodes this day. If you're seeing this warning, it's likely rewards were given out twice this day!")
        } else {
            info!(
                "Updating historical daily uptimes of all nodes and purging old status reports..."
            );
            self.storage
                .update_historical_uptimes(
                    &epoch_iso_8601,
                    &active_monitor_mixnodes,
                    &active_monitor_gateways,
                )
                .await?;
            self.storage.purge_old_statuses(two_days_ago).await?;
        }

        Ok(())
    }

    /// Checks whether there is enough network monitor test run data to distribute rewards
    /// for the specified epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch`: epoch to check
    async fn check_for_monitor_data(&self, epoch: Epoch) -> Result<bool, RewardingError> {
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
                    sleep(self.first_epoch.length()).await;
                    continue;
                }
            };

            // wait's until the start of the *next* epoch, e.g. end of the current chosen epoch
            // (it could be none, for example if we are distributing overdue rewards for the previous epoch)
            // plus add a bit of variance
            if let Some(remaining_time) =
                self.determine_delay_until_next_rewarding(next_rewarding_epoch)
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
            match self.check_for_monitor_data(next_rewarding_epoch).await {
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
