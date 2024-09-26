// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// there is a couple of reasons for putting this in a separate module:
// 1. I didn't feel it fit well in nym contract "cache". It seems like purpose of cache is to just keep updating local data
//    rather than attempting to change global view (i.e. the active set)
//
// 2. However, even if it was to exist in the nym contract cache refresher, we'd have to create a different "run"
//    method as it doesn't have access to the signing client which we need in the case of updating rewarded sets
//    (because nym contract cache can be run by anyone regardless of whether, say, network monitor exists)
//
// 3. Eventually this whole procedure is going to get expanded to allow for distribution of rewarded set generation
//    and hence this might be a good place for it.

use crate::node_describe_cache::DescribedNodes;
use crate::node_status_api::ONE_DAY;
use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::caching::cache::SharedCache;
use crate::support::nyxd::Client;
use crate::support::storage::NymApiStorage;
use error::RewardingError;
pub(crate) use helpers::RewardedNodeWithParams;
use nym_mixnet_contract_common::{CurrentIntervalResponse, Interval};
use nym_task::{TaskClient, TaskManager};
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, trace, warn};

pub(crate) mod error;
mod event_reconciliation;
mod helpers;
mod rewarded_set_assignment;
mod rewarding;
mod transition_beginning;

// naming things is difficult, ok?
// this is struct responsible for advancing an epoch
pub struct EpochAdvancer {
    nyxd_client: Client,
    nym_contract_cache: NymContractCache,
    described_cache: SharedCache<DescribedNodes>,
    storage: NymApiStorage,
}

impl EpochAdvancer {
    pub(crate) async fn current_interval_details(
        &self,
    ) -> Result<CurrentIntervalResponse, RewardingError> {
        Ok(self.nyxd_client.get_current_interval().await?)
    }

    pub(crate) fn new(
        nyxd_client: Client,
        nym_contract_cache: NymContractCache,
        described_cache: SharedCache<DescribedNodes>,
        storage: NymApiStorage,
    ) -> Self {
        EpochAdvancer {
            nyxd_client,
            nym_contract_cache,
            described_cache,
            storage,
        }
    }

    #[allow(clippy::doc_lazy_continuation)]
    // This is where the epoch gets advanced, and all epoch related transactions originate
    // TODO: make sure this is still up to date
    // /// Upon each epoch having finished the following actions are executed by this nym-api:
    // /// 1. it computes the rewards for each node using the ephemera channel for the epoch that
    // ///    ended
    // /// 2. it queries the mixnet contract to check the current `EpochState` in order to figure out whether
    // ///     a different nym-api has already started epoch transition (not yet applicable)
    // /// 3. it sends a `BeginEpochTransition` message to the mixnet contract causing the following to happen:
    // ///     - if successful, the address of this validator is going to be saved as being responsible for progressing this epoch.
    // ///     What it means in practice is that once we have multiple instances of nym-api running,
    // ///     only this one will try to perform the rest of the actions. It will also allow it to
    // ///     more easily recover in case of crashes.
    // ///     - the `EpochState` changes to `Rewarding`, meaning the nym-api will now be allowed to send
    // ///    `RewardNode` transactions. However, it's not going to be able anything else like `ReconcileEpochEvents`
    // ///     until that is done.
    // ///     - ability to send transactions (by other users) that get resolved once given epoch/interval rolls over,
    // ///     such as `BondMixnode` or `DelegateToMixnode` will temporarily be frozen until the entire procedure is finished.
    // /// 4. it obtains the current rewarded set and for each node in there (**SORTED BY NODE_ID!!**),
    // ///    it sends (in a single batch) `RewardMixnode` message with the measured performance.
    // ///    Once the final message gets executed, the mixnet contract automatically transitions
    // ///    the state to `ReconcilingEvents`.
    // /// 5. it obtains the number of pending epoch and interval events and repeatedly sends
    // ///    `ReconcileEpochEvents` transaction until all of them are resolved.
    // ///    At this point the mixnet contract automatically transitions the state to `AdvancingEpoch`.
    // /// 6. it obtains the list of all nodes on the network and pseudorandomly (but weighted by total stake)
    // ///    determines the new rewarded set. It then assigns roles to the provided nodes taking
    // ///    family information into consideration. Finally, it sends `AssignRole` message
    // ///    containing the role assignment information thus (after each role has been assigned)
    // ///    rolling over the epoch and changing the state to `InProgress`.
    // /// 7. it purges old (older than 48h) measurement data
    // /// 8. the whole process repeats once the new epoch finishes
    async fn perform_epoch_operations(&mut self, interval: Interval) -> Result<(), RewardingError> {
        let mut rewards = self.nodes_to_reward(interval).await?;
        rewards.sort_by_key(|a| a.node_id);

        info!("The current epoch has finished.");
        info!(
            "Interval id: {}, epoch id: {} (absolute epoch id: {})",
            interval.current_interval_id(),
            interval.current_epoch_id(),
            interval.current_epoch_absolute_id()
        );
        info!(
            "The current epoch has lasted from {} until {}",
            interval.current_epoch_start(),
            interval.current_epoch_end()
        );

        info!("Performing all epoch operations...");

        let epoch_end = interval.current_epoch_end();

        let legacy_mixnodes = self.nym_contract_cache.legacy_mixnodes_filtered().await;
        let legacy_gateways = self.nym_contract_cache.legacy_gateways_filtered().await;
        let nym_nodes = self.nym_contract_cache.nym_nodes_filtered().await;

        if legacy_mixnodes.is_empty() && legacy_gateways.is_empty() && nym_nodes.is_empty() {
            // that's a bit weird, but ok
            warn!("there don't seem to be any nodes on the network!")
        }

        let epoch_status = self.nyxd_client.get_current_epoch_status().await?;
        if !epoch_status.is_in_progress() {
            if epoch_status.being_advanced_by.as_str()
                != self.nyxd_client.client_address().await.as_ref()
            {
                // another nym-api is already handling
                error!("another nym-api ({}) is already advancing the epoch... but we shouldn't have other nym-apis yet!", epoch_status.being_advanced_by);
                return Ok(());
            } else {
                warn!("we seem to have crashed mid-epoch advancement...");
            }
        } else {
            let should_continue = self.begin_epoch_transition().await?;
            if !should_continue {
                return Ok(());
            }
        }

        // Reward all the nodes in the still current, soon to be previous rewarded set
        info!("Rewarding the current rewarded set...");
        self.reward_current_rewarded_set(rewards, interval).await?;

        // note: those operations don't really have to be atomic, so it's fine to send them
        // as separate transactions
        self.reconcile_epoch_events().await?;
        self.update_rewarded_set_and_advance_epoch(
            interval,
            &legacy_mixnodes,
            &legacy_gateways,
            &nym_nodes,
        )
        .await?;

        info!("Purging old node statuses from the storage...");
        let cutoff = (epoch_end - 2 * ONE_DAY).unix_timestamp();
        self.storage.purge_old_statuses(cutoff).await?;

        Ok(())
    }

    // this purposely does not deal with nym-nodes as they don't have a concept of a blacklist.
    // instead clients are meant to be filtering out them themselves based on the provided scores.
    async fn update_legacy_node_blacklist(
        &mut self,
        interval: &Interval,
    ) -> Result<(), RewardingError> {
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
                gate_blacklist_add.insert(gateway.node_id());
            } else {
                gate_blacklist_remove.insert(gateway.node_id());
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
                info!(
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
        info!("waiting for initial contract cache values before we can start rewarding");
        self.nym_contract_cache.wait_for_initial_values().await;

        info!("waiting for initial self-described cache values before we can start rewarding");
        self.described_cache.naive_wait_for_initial_values().await;

        while !shutdown.is_shutdown() {
            let interval_details = match self.wait_until_epoch_end(&mut shutdown).await {
                // received a shutdown
                None => return Ok(()),
                Some(interval) => interval,
            };
            if let Err(err) = self.update_legacy_node_blacklist(&interval_details).await {
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
        described_cache: SharedCache<DescribedNodes>,
        storage: &NymApiStorage,
        shutdown: &TaskManager,
    ) {
        let mut rewarded_set_updater = EpochAdvancer::new(
            nyxd_client,
            nym_contract_cache.to_owned(),
            described_cache,
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
