// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Deps, DepsMut, Env, Order, StdError, Storage};
use cw_controllers::Admin;
use cw_storage_plus::{Bound, Item, Map};
use nym_mixnet_contract_common::MixnetContractQuerier;
use nym_performance_contract_common::constants::{storage_keys, MAX_FALLBACK_LOOKBACK_EPOCHS};
use nym_performance_contract_common::{
    BatchSubmissionResult, EpochId, EpochNodeMeasurements, EpochNodePerformance, EpochWeights,
    LastSubmission, LastSubmittedData, NetworkMonitorDetails, NetworkMonitorSubmissionMetadata,
    NodeId, NodeSubmission, NymPerformanceContractError, RemoveEpochMeasurementsResponse,
    ResolvedMedians, RetiredNetworkMonitor, RewardingInputsResponse, Weights,
};

pub const NYM_PERFORMANCE_CONTRACT_STORAGE: NymPerformanceContractStorage =
    NymPerformanceContractStorage::new();

pub struct NymPerformanceContractStorage {
    pub(crate) contract_admin: Admin,
    pub(crate) mixnet_epoch_id_at_creation: Item<EpochId>,
    pub(crate) last_performance_submission: Item<LastSubmission>,

    pub(crate) mixnet_contract_address: Item<Addr>,

    pub(crate) network_monitors: NetworkMonitorsStorage,

    pub(crate) performance_results: PerformanceResultsStorage,

    /// Weights keyed by the epoch they take effect from; sparse, written only on change.
    pub(crate) weights: Map<EpochId, Weights>,
}

impl NymPerformanceContractStorage {
    #[allow(clippy::new_without_default)]
    pub(crate) const fn new() -> Self {
        NymPerformanceContractStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
            mixnet_epoch_id_at_creation: Item::new(storage_keys::INITIAL_EPOCH_ID),
            last_performance_submission: Item::new(storage_keys::LAST_SUBMISSION),
            mixnet_contract_address: Item::new(storage_keys::MIXNET_CONTRACT),
            network_monitors: NetworkMonitorsStorage::new(),
            performance_results: PerformanceResultsStorage::new(),
            weights: Map::new(storage_keys::WEIGHTS),
        }
    }

    pub fn current_mixnet_epoch_id(
        &self,
        deps: Deps,
    ) -> Result<EpochId, NymPerformanceContractError> {
        let mixnet_contract_address = self.mixnet_contract_address.load(deps.storage)?;
        let current_epoch_id = deps
            .querier
            .query_current_absolute_mixnet_epoch_id(&mixnet_contract_address)?;
        Ok(current_epoch_id)
    }

    pub fn node_bonded(
        &self,
        deps: Deps,
        node_id: NodeId,
    ) -> Result<bool, NymPerformanceContractError> {
        let mixnet_contract_address = self.mixnet_contract_address.load(deps.storage)?;

        let exists = deps
            .querier
            .check_node_existence(mixnet_contract_address, node_id)?;
        Ok(exists)
    }

    /// Rejects any epoch but the current one, and the current one once its transition has begun.
    fn ensure_writable_epoch(
        &self,
        deps: Deps,
        epoch_id: EpochId,
    ) -> Result<(), NymPerformanceContractError> {
        let mixnet_contract_address = self.mixnet_contract_address.load(deps.storage)?;

        let current_epoch_id = deps
            .querier
            .query_current_absolute_mixnet_epoch_id(&mixnet_contract_address)?;
        if epoch_id != current_epoch_id {
            return Err(NymPerformanceContractError::EpochNotCurrent {
                epoch_id,
                current_epoch_id,
            });
        }

        // the mixnet contract rewards the current epoch inside its transition, so the epoch is
        // sealed the moment the transition begins rather than when the interval advances
        let status = deps
            .querier
            .query_current_mixnet_epoch_status(&mixnet_contract_address)?;
        if !status.is_in_progress() {
            return Err(NymPerformanceContractError::EpochInTransition { epoch_id });
        }

        Ok(())
    }

    pub fn initialise(
        &self,
        mut deps: DepsMut,
        env: Env,
        admin: Addr,
        mixnet_contract_address: Addr,
        initial_authorised_network_monitors: Vec<String>,
        initial_weights: Weights,
    ) -> Result<(), NymPerformanceContractError> {
        // validate before any write so that a rejected instantiation persists nothing of ours
        initial_weights.validate()?;

        // set the mixnet contract address
        self.mixnet_contract_address
            .save(deps.storage, &mixnet_contract_address)?;

        let initial_epoch_id = self.current_mixnet_epoch_id(deps.as_ref())?;

        // set the last submission to the initial value
        self.last_performance_submission.save(
            deps.storage,
            &LastSubmission {
                block_height: env.block.height,
                block_time: env.block.time,
                data: None,
            },
        )?;

        // set the initial epoch id
        self.mixnet_epoch_id_at_creation
            .save(deps.storage, &initial_epoch_id)?;

        // the initial weights are in force from the creation epoch, so no epoch with data lacks weights
        self.weights
            .save(deps.storage, initial_epoch_id, &initial_weights)?;

        // set the contract admin
        self.contract_admin
            .set(deps.branch(), Some(admin.clone()))?;

        // initialise the network monitors storage (by setting the current count to 0)
        self.network_monitors.initialise(deps.branch())?;

        // add all initial network monitors
        for network_monitor in initial_authorised_network_monitors {
            let network_monitor = deps.api.addr_validate(&network_monitor)?;
            self.authorise_network_monitor(deps.branch(), &env, &admin, network_monitor)?;
        }

        Ok(())
    }

    pub fn submit_performance_data(
        &self,
        deps: DepsMut,
        env: Env,
        sender: &Addr,
        epoch_id: EpochId,
        data: NodeSubmission,
    ) -> Result<(), NymPerformanceContractError> {
        // 1. check if the sender is authorised to submit performance data
        self.network_monitors
            .ensure_authorised(deps.storage, sender)?;

        // 2. only the current mixnet epoch accepts data, and only while it is in progress
        self.ensure_writable_epoch(deps.as_ref(), epoch_id)?;

        // 3. check if current submission metadata is consistent with the result we want to submit
        self.performance_results.ensure_non_stale_submission(
            deps.storage,
            sender,
            epoch_id,
            data.node_id,
        )?;

        // 4. a submission without a single measurement would touch a bundle for nothing
        ensure_non_empty(&data)?;

        // 5. check if the node is bonded
        if !self.node_bonded(deps.as_ref(), data.node_id)? {
            return Err(NymPerformanceContractError::NodeNotBonded {
                node_id: data.node_id,
            });
        }

        // 6. insert performance data into the storage
        self.performance_results
            .insert_performance_data(deps.storage, epoch_id, data)?;

        // 7. update submission metadata based on the last result we submitted
        self.performance_results.update_submission_metadata(
            deps.storage,
            sender,
            epoch_id,
            data.node_id,
        )?;

        // 8. update latest submitted
        self.last_performance_submission.save(
            deps.storage,
            &LastSubmission {
                block_height: env.block.height,
                block_time: env.block.time,
                data: Some(LastSubmittedData {
                    sender: sender.clone(),
                    epoch_id,
                    data,
                }),
            },
        )?;

        Ok(())
    }

    pub fn batch_submit_performance_results(
        &self,
        deps: DepsMut,
        env: Env,
        sender: &Addr,
        epoch_id: EpochId,
        data: Vec<NodeSubmission>,
    ) -> Result<BatchSubmissionResult, NymPerformanceContractError> {
        // 1. check if the sender is authorised to submit performance data
        self.network_monitors
            .ensure_authorised(deps.storage, sender)?;

        // 2. only the current mixnet epoch accepts data, and only while it is in progress
        self.ensure_writable_epoch(deps.as_ref(), epoch_id)?;

        // 3. an empty batch has nothing to check or record; otherwise the first entry anchors
        // the cursor check and the last one the cursor update (a single entry is both)
        let (Some(&first), Some(&last)) = (data.first(), data.last()) else {
            return Ok(BatchSubmissionResult::default());
        };

        // 4. check if current submission metadata is consistent with the first result we want to submit
        self.performance_results.ensure_non_stale_submission(
            deps.storage,
            sender,
            epoch_id,
            first.node_id,
        )?;

        let mut accepted_scores = 0;
        let mut non_existent_nodes = Vec::new();
        let mut previous: Option<NodeId> = None;

        // 5. per node: the payload must carry something, node ids must strictly ascend, and only
        // bonded nodes are stored (a failed check reverts the whole tx, so earlier writes are fine)
        for submission in data {
            ensure_non_empty(&submission)?;
            if previous.is_some_and(|previous| submission.node_id <= previous) {
                return Err(NymPerformanceContractError::UnsortedBatchSubmission);
            }
            previous = Some(submission.node_id);

            if self.node_bonded(deps.as_ref(), submission.node_id)? {
                self.performance_results.insert_performance_data(
                    deps.storage,
                    epoch_id,
                    submission,
                )?;
                accepted_scores += 1;
            } else {
                non_existent_nodes.push(submission.node_id);
            }
        }

        // 6. update submission metadata based on the last result we submitted
        self.performance_results.update_submission_metadata(
            deps.storage,
            sender,
            epoch_id,
            last.node_id,
        )?;

        // 7. update latest submitted
        self.last_performance_submission.save(
            deps.storage,
            &LastSubmission {
                block_height: env.block.height,
                block_time: env.block.time,
                data: Some(LastSubmittedData {
                    sender: sender.clone(),
                    epoch_id,
                    data: last,
                }),
            },
        )?;

        Ok(BatchSubmissionResult {
            accepted_scores,
            non_existent_nodes,
        })
    }

    #[cfg(test)]
    fn is_admin(&self, deps: Deps, addr: &Addr) -> Result<bool, NymPerformanceContractError> {
        self.contract_admin.is_admin(deps, addr).map_err(Into::into)
    }

    fn ensure_is_admin(&self, deps: Deps, addr: &Addr) -> Result<(), NymPerformanceContractError> {
        self.contract_admin
            .assert_admin(deps, addr)
            .map_err(Into::into)
    }

    pub fn authorise_network_monitor(
        &self,
        mut deps: DepsMut,
        env: &Env,
        sender: &Addr,
        network_monitor: Addr,
    ) -> Result<(), NymPerformanceContractError> {
        self.ensure_is_admin(deps.as_ref(), sender)?;

        // make sure this address is not already authorised (it'd mess up the total count)
        if self
            .network_monitors
            .authorised
            .has(deps.storage, &network_monitor)
        {
            return Err(NymPerformanceContractError::AlreadyAuthorised {
                address: network_monitor,
            });
        }

        // insert the new entry and adjust the total count
        self.network_monitors
            .insert_new(deps.branch(), env, sender, &network_monitor)?;

        // finally, set submission metadata to disallow this NM from submitting data for epochs before it was authorised
        let current_epoch_id = self.current_mixnet_epoch_id(deps.as_ref())?;

        self.performance_results.submission_metadata.save(
            deps.storage,
            &network_monitor,
            &NetworkMonitorSubmissionMetadata {
                last_submitted_epoch_id: current_epoch_id,
                last_submitted_node_id: 0,
            },
        )?;
        Ok(())
    }

    pub fn retire_network_monitor(
        &self,
        deps: DepsMut,
        env: Env,
        sender: &Addr,
        network_monitor: Addr,
    ) -> Result<(), NymPerformanceContractError> {
        self.ensure_is_admin(deps.as_ref(), sender)?;

        self.network_monitors
            .retire(deps, &env, sender, &network_monitor)
    }

    /// Stores weights that take effect from the next mixnet epoch and returns that epoch.
    pub fn update_weights(
        &self,
        deps: DepsMut,
        sender: &Addr,
        weights: Weights,
    ) -> Result<EpochId, NymPerformanceContractError> {
        self.ensure_is_admin(deps.as_ref(), sender)?;
        weights.validate()?;

        // an update made mid-epoch applies from the next epoch onwards, so every epoch's weights
        // were fixed before that epoch began; two updates in one epoch simply overwrite
        let effective_from = self.current_mixnet_epoch_id(deps.as_ref())? + 1;
        self.weights.save(deps.storage, effective_from, &weights)?;
        Ok(effective_from)
    }

    /// The weights in force at `epoch_id`: the latest entry at or before it, if any.
    pub fn weights_at(
        &self,
        storage: &dyn Storage,
        epoch_id: EpochId,
    ) -> Result<Option<EpochWeights>, NymPerformanceContractError> {
        let latest = self
            .weights
            .range(
                storage,
                None,
                Some(Bound::inclusive(epoch_id)),
                Order::Descending,
            )
            .next()
            .transpose()?;

        Ok(latest.map(|(effective_from, weights)| EpochWeights {
            effective_from,
            weights,
        }))
    }

    /// Everything rewarding uses for `node_id` in `epoch_id`: the bundle for that epoch, or the
    /// newest earlier one within the lookback, scored with the weights in force at `epoch_id`.
    /// Storage only, so repeated calls over frozen data return identical answers.
    pub fn resolve_rewarding_inputs(
        &self,
        storage: &dyn Storage,
        epoch_id: EpochId,
        node_id: NodeId,
    ) -> Result<RewardingInputsResponse, NymPerformanceContractError> {
        let source = self.resolve_source_bundle(storage, epoch_id, node_id)?;
        let weights = self.weights_at(storage, epoch_id)?;
        let score = source
            .as_ref()
            .zip(weights.as_ref())
            .and_then(|(source, weights)| weights.weights.score(source.medians));

        Ok(RewardingInputsResponse {
            requested_epoch_id: epoch_id,
            source,
            weights,
            score,
        })
    }

    /// The bundle at `epoch_id`, or the newest earlier one within `MAX_FALLBACK_LOOKBACK_EPOCHS`.
    fn resolve_source_bundle(
        &self,
        storage: &dyn Storage,
        epoch_id: EpochId,
        node_id: NodeId,
    ) -> Result<Option<ResolvedMedians>, NymPerformanceContractError> {
        let results = &self.performance_results.results;

        if let Some(bundle) = results.may_load(storage, (epoch_id, node_id))? {
            return Ok(Some(ResolvedMedians {
                epoch_id,
                medians: bundle.medians(),
            }));
        }

        let Some(last_known) = self
            .performance_results
            .last_known_epoch
            .may_load(storage, node_id)?
        else {
            return Ok(None);
        };
        let floor = epoch_id.saturating_sub(MAX_FALLBACK_LOOKBACK_EPOCHS);

        if last_known <= epoch_id {
            // the pointer names the newest bundle the node ever had, so one load settles it; if
            // that bundle is gone an admin removed it (removals never touch the pointer), and
            // nothing older would be any more legitimate to reward on, so there is nothing to walk
            if last_known < floor {
                return Ok(None);
            }
            return Ok(results
                .may_load(storage, (last_known, node_id))?
                .map(|bundle| ResolvedMedians {
                    epoch_id: last_known,
                    medians: bundle.medians(),
                }));
        }

        // data arrived after the requested epoch, so the pointer says nothing about what lies
        // below it; the newest bundle below the request is what rewarding saw at the time, and
        // only a walk finds it again
        for candidate in (floor..epoch_id).rev() {
            if let Some(bundle) = results.may_load(storage, (candidate, node_id))? {
                return Ok(Some(ResolvedMedians {
                    epoch_id: candidate,
                    medians: bundle.medians(),
                }));
            }
        }

        Ok(None)
    }

    /// The medians of the bundle at exactly `(epoch_id, node_id)` and their score under that
    /// epoch's weights; never falls back to another epoch.
    pub fn try_load_performance(
        &self,
        storage: &dyn Storage,
        epoch_id: EpochId,
        node_id: NodeId,
    ) -> Result<Option<EpochNodePerformance>, NymPerformanceContractError> {
        let Some(bundle) = self
            .performance_results
            .results
            .may_load(storage, (epoch_id, node_id))?
        else {
            return Ok(None);
        };

        let medians = bundle.medians();
        let score = self
            .weights_at(storage, epoch_id)?
            .and_then(|weights| weights.weights.score(medians));

        Ok(Some(EpochNodePerformance {
            epoch_id,
            medians,
            score,
        }))
    }

    /// Removes one bundle; the node's last-known epoch and the weights are left alone.
    pub fn remove_node_measurements(
        &self,
        deps: DepsMut,
        sender: &Addr,
        epoch_id: EpochId,
        node_id: NodeId,
    ) -> Result<(), NymPerformanceContractError> {
        self.ensure_is_admin(deps.as_ref(), sender)?;

        self.performance_results
            .results
            .remove(deps.storage, (epoch_id, node_id));
        Ok(())
    }

    /// Removes up to the purge limit of an epoch's bundles; pointers and weights are left alone.
    pub fn remove_epoch_measurements(
        &self,
        deps: DepsMut,
        sender: &Addr,
        epoch_id: EpochId,
    ) -> Result<RemoveEpochMeasurementsResponse, NymPerformanceContractError> {
        self.ensure_is_admin(deps.as_ref(), sender)?;

        // 1. purge the entries according to the limit
        self.performance_results.results.prefix(epoch_id).clear(
            deps.storage,
            Some(retrieval_limits::EPOCH_PERFORMANCE_PURGE_LIMIT),
        );

        // 2. see if there's anything left
        let additional_entries_to_remove_remaining = !self
            .performance_results
            .results
            .prefix(epoch_id)
            .is_empty(deps.storage);

        Ok(RemoveEpochMeasurementsResponse {
            additional_entries_to_remove_remaining,
        })
    }
}

/// A submission with every kind absent would create or touch a bundle for nothing.
fn ensure_non_empty(submission: &NodeSubmission) -> Result<(), NymPerformanceContractError> {
    if submission.measurements.is_empty() {
        return Err(NymPerformanceContractError::EmptyNodeSubmission {
            node_id: submission.node_id,
        });
    }
    Ok(())
}

pub(crate) struct NetworkMonitorsStorage {
    pub(crate) authorised_count: Item<u32>,
    pub(crate) authorised: Map<&'static Addr, NetworkMonitorDetails>,
    pub(crate) retired: Map<&'static Addr, RetiredNetworkMonitor>,
}

impl NetworkMonitorsStorage {
    #[allow(clippy::new_without_default)]
    const fn new() -> Self {
        NetworkMonitorsStorage {
            authorised_count: Item::new(storage_keys::AUTHORISED_COUNT),
            authorised: Map::new(storage_keys::AUTHORISED),
            retired: Map::new(storage_keys::RETIRED),
        }
    }

    fn initialise(&self, deps: DepsMut) -> Result<(), NymPerformanceContractError> {
        self.authorised_count.save(deps.storage, &0)?;
        Ok(())
    }

    fn ensure_authorised(
        &self,
        storage: &dyn Storage,
        addr: &Addr,
    ) -> Result<(), NymPerformanceContractError> {
        if !self.authorised.has(storage, addr) {
            return Err(NymPerformanceContractError::NotAuthorised {
                address: addr.clone(),
            });
        }
        Ok(())
    }

    fn insert_new(
        &self,
        deps: DepsMut,
        env: &Env,
        sender: &Addr,
        address: &Addr,
    ) -> Result<(), NymPerformanceContractError> {
        // if this address has already been retired in the past, restore it
        self.retired.remove(deps.storage, address);

        self.authorised_count
            .update(deps.storage, |authorised_count| {
                Ok::<_, StdError>(authorised_count + 1)
            })?;
        self.authorised.save(
            deps.storage,
            address,
            &NetworkMonitorDetails {
                address: address.clone(),
                authorised_by: sender.clone(),
                authorised_at_height: env.block.height,
            },
        )?;
        Ok(())
    }

    fn retire(
        &self,
        deps: DepsMut,
        env: &Env,
        sender: &Addr,
        addr: &Addr,
    ) -> Result<(), NymPerformanceContractError> {
        // NOTE: if the NM hasn't been authorised before, the `load` call will fail
        // and thus `authorised_count` won't be updated (nor further code executed)
        let details = self.authorised.load(deps.storage, addr)?;
        self.authorised.remove(deps.storage, addr);

        self.authorised_count
            .update(deps.storage, |authorised_count| {
                Ok::<_, StdError>(authorised_count - 1)
            })?;

        self.retired
            .save(deps.storage, addr, &details.retire(env, sender))?;
        Ok(())
    }
}

pub(crate) struct PerformanceResultsStorage {
    /// One bundle per (epoch, node) holding every monitor's values for every kind.
    pub(crate) results: Map<(EpochId, NodeId), EpochNodeMeasurements>,

    /// The latest epoch each node has a bundle for; written when a bundle is created.
    pub(crate) last_known_epoch: Map<NodeId, EpochId>,

    // in order to ensure NM does not resubmit results, we keep metadata
    // of the latest submitted information
    // this requires them to submit everything sorted by node_id
    pub(crate) submission_metadata: Map<&'static Addr, NetworkMonitorSubmissionMetadata>,
}

impl PerformanceResultsStorage {
    #[allow(clippy::new_without_default)]
    const fn new() -> Self {
        PerformanceResultsStorage {
            results: Map::new(storage_keys::PERFORMANCE_RESULTS),
            last_known_epoch: Map::new(storage_keys::LAST_KNOWN_EPOCH),
            submission_metadata: Map::new(storage_keys::SUBMISSION_METADATA),
        }
    }

    // note: this method assumes authorisation has been checked and invariants validated
    // (such as attempting to insert stale data or an empty submission)
    fn insert_performance_data(
        &self,
        storage: &mut dyn Storage,
        epoch_id: EpochId,
        submission: NodeSubmission,
    ) -> Result<(), NymPerformanceContractError> {
        let key = (epoch_id, submission.node_id);
        let bundle = match self.results.may_load(storage, key)? {
            Some(mut existing) => {
                existing.insert(submission.measurements);
                existing
            }
            None => {
                // the first monitor to report this node in this epoch creates the bundle,
                // which is the one moment the node's last-known epoch can move
                self.advance_last_known_epoch(storage, submission.node_id, epoch_id)?;
                EpochNodeMeasurements::new(submission.measurements)
            }
        };

        self.results.save(storage, key, &bundle)?;
        Ok(())
    }

    /// Moves the node's last-known epoch forward, never back.
    fn advance_last_known_epoch(
        &self,
        storage: &mut dyn Storage,
        node_id: NodeId,
        epoch_id: EpochId,
    ) -> Result<(), NymPerformanceContractError> {
        self.last_known_epoch.update(storage, node_id, |current| {
            Ok::<_, StdError>(current.map_or(epoch_id, |current| current.max(epoch_id)))
        })?;
        Ok(())
    }

    fn update_submission_metadata(
        &self,
        storage: &mut dyn Storage,
        address: &Addr,
        last_submitted_epoch_id: EpochId,
        last_submitted_node_id: NodeId,
    ) -> Result<(), NymPerformanceContractError> {
        self.submission_metadata.save(
            storage,
            address,
            &NetworkMonitorSubmissionMetadata {
                last_submitted_epoch_id,
                last_submitted_node_id,
            },
        )?;
        Ok(())
    }

    fn ensure_non_stale_submission(
        &self,
        storage: &dyn Storage,
        address: &Addr,
        epoch_id: EpochId,
        new_node_id: NodeId,
    ) -> Result<(), NymPerformanceContractError> {
        let last_submission = self.submission_metadata.load(storage, address)?;

        // we can't submit data for past epochs
        if last_submission.last_submitted_epoch_id > epoch_id {
            return Err(NymPerformanceContractError::StalePerformanceSubmission {
                epoch_id,
                node_id: new_node_id,
                last_epoch_id: last_submission.last_submitted_epoch_id,
                last_node_id: last_submission.last_submitted_node_id,
            });
        }

        // if we're submitting for the same epoch, the node id has to be greater than the previous one
        if last_submission.last_submitted_epoch_id == epoch_id
            && last_submission.last_submitted_node_id >= new_node_id
        {
            return Err(NymPerformanceContractError::StalePerformanceSubmission {
                epoch_id,
                node_id: new_node_id,
                last_epoch_id: last_submission.last_submitted_epoch_id,
                last_node_id: last_submission.last_submitted_node_id,
            });
        }
        // if we're submitting for new epoch, node id doesn't matter
        Ok(())
    }
}

pub mod retrieval_limits {
    pub const NODE_PERFORMANCE_DEFAULT_LIMIT: u32 = 100;
    pub const NODE_PERFORMANCE_MAX_LIMIT: u32 = 200;

    pub const NODE_EPOCH_PERFORMANCE_DEFAULT_LIMIT: u32 = 100;
    pub const NODE_EPOCH_PERFORMANCE_MAX_LIMIT: u32 = 200;

    pub const NODE_EPOCH_MEASUREMENTS_DEFAULT_LIMIT: u32 = 50;
    pub const NODE_EPOCH_MEASUREMENTS_MAX_LIMIT: u32 = 100;

    pub const NODE_HISTORICAL_PERFORMANCE_DEFAULT_LIMIT: u32 = 100;
    pub const NODE_HISTORICAL_PERFORMANCE_MAX_LIMIT: u32 = 200;

    pub const NETWORK_MONITORS_DEFAULT_LIMIT: u32 = 50;
    pub const NETWORK_MONITORS_MAX_LIMIT: u32 = 100;

    pub const RETIRED_NETWORK_MONITORS_DEFAULT_LIMIT: u32 = 50;
    pub const RETIRED_NETWORK_MONITORS_MAX_LIMIT: u32 = 100;

    pub const EPOCH_PERFORMANCE_PURGE_LIMIT: usize = 200;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod performance_contract_storage {
        use super::*;
        use crate::testing::{
            init_contract_tester, liveness_submission, p, scored_submission, values,
            PerformanceContractTesterExt, PreInitContract,
        };
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use nym_contracts_common_testing::{AdminExt, ContractOpts};
        use nym_performance_contract_common::{KindMedians, Measurements};

        #[cfg(test)]
        mod initialisation {
            use super::*;
            use crate::testing::{liveness_only_weights, p};
            use cosmwasm_std::{Decimal, Order, StdResult};
            use nym_contracts_common::Percent;
            use nym_contracts_common_testing::{ArbitraryContractStorageWriter, FullReader};

            fn initialise_storage(
                pre_init: &mut PreInitContract,
                admin: Option<Addr>,
            ) -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mixnet_contract = pre_init.mixnet_contract_address.clone();
                let env = pre_init.env();
                let admin = admin.unwrap_or(pre_init.addr_make("admin"));
                let deps = pre_init.deps_mut();

                storage.initialise(
                    deps,
                    env,
                    admin,
                    mixnet_contract.clone(),
                    Vec::new(),
                    liveness_only_weights(),
                )?;
                Ok(())
            }

            #[test]
            fn stores_initial_weights_under_the_creation_epoch() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut pre_init = PreInitContract::new();
                let address = pre_init.mixnet_contract_address.clone();

                // a non-zero creation epoch proves the key is the creation epoch, not a constant
                let mut interval = pre_init.querier().query_current_mixnet_interval(&address)?;
                for _ in 0..7 {
                    interval = interval.advance_epoch();
                }
                pre_init.set_contract_storage_value(&address, b"ci", &interval)?;

                initialise_storage(&mut pre_init, None)?;
                let deps = pre_init.deps();

                let stored = storage
                    .weights
                    .range(deps.storage, None, None, Order::Ascending)
                    .collect::<StdResult<Vec<_>>>()?;
                assert_eq!(stored, vec![(7, liveness_only_weights())]);

                Ok(())
            }

            #[test]
            fn rejects_invalid_initial_weights_before_writing_anything() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut pre_init = PreInitContract::new();
                let mixnet_contract = pre_init.mixnet_contract_address.clone();
                let env = pre_init.env();
                let admin = pre_init.addr_make("admin");

                let invalid = Weights {
                    liveness: p("0.7"),
                    stress: Percent::zero(),
                };
                let res = storage
                    .initialise(
                        pre_init.deps_mut(),
                        env,
                        admin,
                        mixnet_contract,
                        Vec::new(),
                        invalid,
                    )
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::WeightsDoNotSumToOne {
                        total: Decimal::percent(70)
                    }
                );

                let deps = pre_init.deps();
                assert!(storage.weights.is_empty(deps.storage));
                assert!(storage
                    .mixnet_contract_address
                    .may_load(deps.storage)?
                    .is_none());
                // `Admin::get` errors on a never-written item, so check the raw key instead
                assert!(deps
                    .storage
                    .get(storage_keys::CONTRACT_ADMIN.as_bytes())
                    .is_none());

                Ok(())
            }

            #[test]
            fn sets_contract_admin() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut pre_init = PreInitContract::new();
                let admin1 = pre_init.api.addr_make("first-admin");
                let admin2 = pre_init.api.addr_make("second-admin");

                initialise_storage(&mut pre_init, Some(admin1.clone()))?;
                let deps = pre_init.deps();
                assert!(storage.ensure_is_admin(deps, &admin1).is_ok());

                let mut pre_init = PreInitContract::new();
                initialise_storage(&mut pre_init, Some(admin2.clone()))?;
                let deps = pre_init.deps();
                assert!(storage.ensure_is_admin(deps, &admin2).is_ok());

                Ok(())
            }

            #[test]
            fn sets_provided_mixnet_contract_address() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut pre_init = PreInitContract::new();

                initialise_storage(&mut pre_init, None)?;

                let expected_mixnet_contract_address = pre_init.mixnet_contract_address.clone();
                let deps = pre_init.deps();
                let mixnet_contract = storage.mixnet_contract_address.load(deps.storage)?;
                assert_eq!(expected_mixnet_contract_address, mixnet_contract);
                Ok(())
            }

            #[test]
            fn sets_initial_submission_data() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut pre_init = PreInitContract::new();

                let env = pre_init.env();
                initialise_storage(&mut pre_init, None)?;
                let deps = pre_init.deps();

                let expected = LastSubmission {
                    block_height: env.block.height,
                    block_time: env.block.time,
                    data: None,
                };
                let data = storage.last_performance_submission.load(deps.storage)?;
                assert_eq!(expected, data);
                Ok(())
            }

            #[test]
            fn retrieves_initial_epoch_id_from_mixnet_contract() -> anyhow::Result<()> {
                // base case
                let storage = NymPerformanceContractStorage::new();
                let mut pre_init = PreInitContract::new();

                initialise_storage(&mut pre_init, None)?;
                let deps = pre_init.deps();
                assert_eq!(0, storage.mixnet_epoch_id_at_creation.load(deps.storage)?);

                // non-0 epoch
                let storage = NymPerformanceContractStorage::new();
                let mut pre_init = PreInitContract::new();

                let address = pre_init.mixnet_contract_address.clone();

                // advance the epoch few times...
                let interval_details = pre_init
                    .querier()
                    .query_current_mixnet_interval(&address)?
                    .advance_epoch()
                    .advance_epoch()
                    .advance_epoch()
                    .advance_epoch()
                    .advance_epoch()
                    .advance_epoch()
                    .advance_epoch();

                pre_init.set_contract_storage_value(&address, b"ci", &interval_details)?;

                initialise_storage(&mut pre_init, None)?;
                let deps = pre_init.deps();
                assert_eq!(7, storage.mixnet_epoch_id_at_creation.load(deps.storage)?);

                Ok(())
            }

            #[test]
            fn authorises_provided_network_monitors() -> anyhow::Result<()> {
                // no NM
                let storage = NymPerformanceContractStorage::new();
                let mut pre_init = PreInitContract::new();

                initialise_storage(&mut pre_init, None)?;
                let deps = pre_init.deps();
                let authorised_count = storage
                    .network_monitors
                    .authorised_count
                    .load(deps.storage)?;
                assert_eq!(authorised_count, 0);

                let authorised = storage
                    .network_monitors
                    .authorised
                    .all_values(deps.storage)?;
                assert!(authorised.is_empty());

                let mut pre_init = PreInitContract::new();
                let mixnet_contract = pre_init.mixnet_contract_address.clone();
                let env = pre_init.env();
                let admin = pre_init.addr_make("admin");
                let nm1 = pre_init.addr_make("nm1");
                let nm2 = pre_init.addr_make("nm2");

                let deps = pre_init.deps_mut();
                storage.initialise(
                    deps,
                    env.clone(),
                    admin.clone(),
                    mixnet_contract.clone(),
                    vec![nm1.to_string(), nm2.to_string()],
                    liveness_only_weights(),
                )?;

                let deps = pre_init.deps();
                let authorised_count = storage
                    .network_monitors
                    .authorised_count
                    .load(deps.storage)?;
                assert_eq!(authorised_count, 2);

                let authorised = storage
                    .network_monitors
                    .authorised
                    .all_values(deps.storage)?;

                let expected = vec![
                    NetworkMonitorDetails {
                        address: nm1,
                        authorised_by: admin.clone(),
                        authorised_at_height: env.block.height,
                    },
                    NetworkMonitorDetails {
                        address: nm2,
                        authorised_by: admin.clone(),
                        authorised_at_height: env.block.height,
                    },
                ];
                assert_eq!(authorised, expected);

                Ok(())
            }
        }

        #[test]
        fn getting_current_mixnet_epoch_id() -> anyhow::Result<()> {
            let storage = NymPerformanceContractStorage::new();
            let mut tester = init_contract_tester();

            assert_eq!(storage.current_mixnet_epoch_id(tester.deps())?, 0);
            tester.advance_mixnet_epoch()?;
            assert_eq!(storage.current_mixnet_epoch_id(tester.deps())?, 1);

            tester.set_mixnet_epoch(1000)?;
            assert_eq!(storage.current_mixnet_epoch_id(tester.deps())?, 1000);

            Ok(())
        }

        #[cfg(test)]
        mod submitting_performance_data {
            use super::*;
            use nym_mixnet_contract_common::nym_node::Role;
            use nym_mixnet_contract_common::EpochState;

            #[test]
            fn rejects_a_submission_without_any_measurement() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm = tester.new_authorised_network_monitor();
                let env = tester.env();

                // the payload is checked before the node is, so it need not even be bonded
                let empty = NodeSubmission {
                    node_id: 12345,
                    measurements: Measurements::default(),
                };
                let res = storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &nm, 0, empty)
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::EmptyNodeSubmission { node_id: 12345 }
                );

                // nothing was written and the cursor was not advanced
                assert!(storage
                    .performance_results
                    .results
                    .may_load(&tester, (0, 12345))?
                    .is_none());
                let metadata = tester.submission_metadata(&nm);
                assert_eq!(metadata.last_submitted_node_id, 0);

                // which a lower node id proves: it would be stale had the cursor moved to 12345
                let data = tester.dummy_node_submission();
                assert!(data.node_id < 12345);
                storage.submit_performance_data(tester.deps_mut(), env, &nm, 0, data)?;

                Ok(())
            }

            #[test]
            fn is_only_allowed_by_authorised_network_monitors() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm1 = tester.addr_make("network-monitor-1");
                let nm2 = tester.addr_make("network-monitor-2");
                let unauthorised = tester.addr_make("unauthorised");
                let env = tester.env();

                tester.authorise_network_monitor(&nm1);

                // authorised network monitor can submit the results just fine
                let perf = tester.dummy_node_submission();
                assert!(storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &nm1, 0, perf)
                    .is_ok());

                // unauthorised address is rejected
                let res = storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &nm2, 0, perf)
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::NotAuthorised {
                        address: nm2.clone()
                    }
                );

                // it is fine after explicit authorisation though
                tester.authorise_network_monitor(&nm2);
                assert!(storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &nm2, 0, perf)
                    .is_ok());

                // and address that was never authorised still fails
                let res = storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &unauthorised, 0, perf)
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::NotAuthorised {
                        address: unauthorised
                    }
                );
                Ok(())
            }

            #[test]
            fn its_not_possible_to_submit_data_for_same_node_again() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let env = tester.env();
                let nm = tester.new_authorised_network_monitor();

                let id1 = tester.bond_dummy_nymnode()?;
                let id2 = tester.bond_dummy_nymnode()?;

                let data = liveness_submission(id1, "1");
                let another_data = liveness_submission(id2, "1");

                // first submission
                assert!(storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &nm, 0, data)
                    .is_ok());

                // second submission
                let res = storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &nm, 0, data)
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::StalePerformanceSubmission {
                        epoch_id: 0,
                        node_id: id1,
                        last_epoch_id: 0,
                        last_node_id: id1,
                    }
                );

                // another submission works fine
                assert!(storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &nm, 0, another_data)
                    .is_ok());

                // original one works IF it's for the next epoch, once that epoch is current
                tester.set_mixnet_epoch(1)?;
                assert!(storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &nm, 1, data)
                    .is_ok());

                // the past epoch is now rejected by the epoch check, before the cursor is consulted
                let res = storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &nm, 0, data)
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::EpochNotCurrent {
                        epoch_id: 0,
                        current_epoch_id: 1,
                    }
                );

                Ok(())
            }

            #[test]
            fn its_not_possible_to_submit_data_out_of_order() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm = tester.new_authorised_network_monitor();
                let env = tester.env();

                let id1 = tester.bond_dummy_nymnode()?;
                let id2 = tester.bond_dummy_nymnode()?;
                let data = liveness_submission(id1, "1");
                let another_data = liveness_submission(id2, "1");

                assert!(storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &nm, 0, another_data)
                    .is_ok());

                let res = storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &nm, 0, data)
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::StalePerformanceSubmission {
                        epoch_id: 0,
                        node_id: id1,
                        last_epoch_id: 0,
                        last_node_id: id2,
                    }
                );

                // check across epochs: a new epoch resets the node ordering
                tester.set_mixnet_epoch(10)?;
                assert!(storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &nm, 10, data)
                    .is_ok());

                // and an earlier epoch is rejected before the cursor is consulted
                let res = storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &nm, 9, data)
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::EpochNotCurrent {
                        epoch_id: 9,
                        current_epoch_id: 10,
                    }
                );
                Ok(())
            }

            #[test]
            fn its_only_possible_to_submit_data_for_the_current_epoch() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                tester.set_mixnet_epoch(10)?;

                let nm = tester.new_authorised_network_monitor();
                let env = tester.env();
                let data = tester.dummy_node_submission();

                // past epochs are rejected before the cursor is even consulted
                for past in [0, 9] {
                    let res = storage
                        .submit_performance_data(tester.deps_mut(), env.clone(), &nm, past, data)
                        .unwrap_err();
                    assert_eq!(
                        res,
                        NymPerformanceContractError::EpochNotCurrent {
                            epoch_id: past,
                            current_epoch_id: 10,
                        }
                    );
                }

                // and so are future ones
                let res = storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &nm, 11, data)
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::EpochNotCurrent {
                        epoch_id: 11,
                        current_epoch_id: 10,
                    }
                );

                // the current epoch is accepted
                storage.submit_performance_data(tester.deps_mut(), env.clone(), &nm, 10, data)?;

                // and the next one only once the mixnet has moved on
                tester.set_mixnet_epoch(11)?;
                storage.submit_performance_data(tester.deps_mut(), env, &nm, 11, data)?;

                Ok(())
            }

            #[test]
            fn its_not_possible_to_submit_data_during_the_epoch_transition() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let nm = tester.new_authorised_network_monitor();
                let env = tester.env();
                let data = tester.dummy_node_submission();

                // every stage of the transition seals the epoch being advanced
                for state in [
                    EpochState::Rewarding {
                        last_rewarded: 0,
                        final_node_id: 42,
                    },
                    EpochState::ReconcilingEvents,
                    EpochState::RoleAssignment { next: Role::Layer1 },
                ] {
                    tester.set_mixnet_epoch_status(state);
                    let res = storage
                        .submit_performance_data(tester.deps_mut(), env.clone(), &nm, 0, data)
                        .unwrap_err();
                    assert_eq!(
                        res,
                        NymPerformanceContractError::EpochInTransition { epoch_id: 0 }
                    );
                }

                // nothing was written and the cursor was not advanced
                assert!(storage
                    .performance_results
                    .results
                    .may_load(&tester, (0, data.node_id))?
                    .is_none());
                let metadata = tester.submission_metadata(&nm);
                assert_eq!(metadata.last_submitted_node_id, 0);

                // once the transition has run its course the new epoch accepts data
                tester.set_mixnet_epoch_status(EpochState::InProgress);
                tester.advance_mixnet_epoch()?;
                storage.submit_performance_data(tester.deps_mut(), env, &nm, 1, data)?;

                Ok(())
            }

            #[test]
            fn bundles_are_frozen_once_the_epoch_advances() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                tester.set_mixnet_epoch(10)?;

                let nm1 = tester.addr_make("network-monitor-1");
                let nm2 = tester.addr_make("network-monitor-2");
                tester.authorise_network_monitor(&nm1);
                tester.authorise_network_monitor(&nm2);
                let env = tester.env();
                let data = tester.dummy_node_submission();

                storage.submit_performance_data(tester.deps_mut(), env.clone(), &nm1, 10, data)?;
                tester.set_mixnet_epoch(11)?;

                // the second monitor is too late: epoch 10 can never change again
                let res = storage
                    .submit_performance_data(tester.deps_mut(), env, &nm2, 10, data)
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::EpochNotCurrent {
                        epoch_id: 10,
                        current_epoch_id: 11,
                    }
                );

                let bundle = tester.read_bundle(10, data.node_id);
                assert_eq!(values(bundle.liveness.as_ref().unwrap()), vec![p("0.69")]);
                assert!(bundle.stress.is_none());
                assert!(bundle.config.is_none());

                Ok(())
            }

            #[test]
            fn updates_submission_metadata() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let env = tester.env();

                let nodes = tester.bond_dummy_nymnodes(10);

                let nm = tester.new_authorised_network_monitor();
                let metadata = tester.submission_metadata(&nm);
                assert_eq!(metadata.last_submitted_epoch_id, 0);
                assert_eq!(metadata.last_submitted_node_id, 0);

                storage.submit_performance_data(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    0,
                    liveness_submission(nodes[0], "0"),
                )?;
                let metadata = tester.submission_metadata(&nm);
                assert_eq!(metadata.last_submitted_epoch_id, 0);
                assert_eq!(metadata.last_submitted_node_id, nodes[0]);

                storage.submit_performance_data(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    0,
                    liveness_submission(nodes[3], "0"),
                )?;
                let metadata = tester.submission_metadata(&nm);
                assert_eq!(metadata.last_submitted_epoch_id, 0);
                assert_eq!(metadata.last_submitted_node_id, nodes[3]);

                // a new epoch has to be current before it accepts data
                tester.set_mixnet_epoch(1)?;
                storage.submit_performance_data(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    1,
                    liveness_submission(nodes[1], "0"),
                )?;
                let metadata = tester.submission_metadata(&nm);
                assert_eq!(metadata.last_submitted_epoch_id, 1);
                assert_eq!(metadata.last_submitted_node_id, nodes[1]);

                tester.set_mixnet_epoch(12345)?;
                storage.submit_performance_data(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    12345,
                    liveness_submission(nodes[8], "0"),
                )?;
                let metadata = tester.submission_metadata(&nm);
                assert_eq!(metadata.last_submitted_epoch_id, 12345);
                assert_eq!(metadata.last_submitted_node_id, nodes[8]);

                Ok(())
            }

            #[test]
            fn updates_latest_submitted_information() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let env = tester.env();

                let nm = tester.new_authorised_network_monitor();

                let nodes = tester.bond_dummy_nymnodes(10);

                let expected = |epoch_id: EpochId, data: NodeSubmission| LastSubmission {
                    block_height: env.block.height,
                    block_time: env.block.time,
                    data: Some(LastSubmittedData {
                        sender: nm.clone(),
                        epoch_id,
                        data,
                    }),
                };

                let data = liveness_submission(nodes[0], "0");
                storage.submit_performance_data(tester.deps_mut(), env.clone(), &nm, 0, data)?;
                assert_eq!(
                    storage.last_performance_submission.load(&tester)?,
                    expected(0, data)
                );

                let data = liveness_submission(nodes[6], "0");
                storage.submit_performance_data(tester.deps_mut(), env.clone(), &nm, 0, data)?;
                assert_eq!(
                    storage.last_performance_submission.load(&tester)?,
                    expected(0, data)
                );

                tester.set_mixnet_epoch(1)?;
                let data = liveness_submission(nodes[2], "0");
                storage.submit_performance_data(tester.deps_mut(), env.clone(), &nm, 1, data)?;
                assert_eq!(
                    storage.last_performance_submission.load(&tester)?,
                    expected(1, data)
                );

                tester.set_mixnet_epoch(12345)?;
                let data = liveness_submission(nodes[9], "0");
                storage.submit_performance_data(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    12345,
                    data,
                )?;
                assert_eq!(
                    storage.last_performance_submission.load(&tester)?,
                    expected(12345, data)
                );

                Ok(())
            }

            #[test]
            fn requires_associated_node_to_be_bonded() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let env = tester.env();

                let nm = tester.new_authorised_network_monitor();

                let dummy_perf = liveness_submission(12345, "0.69");

                // no node bonded at this point
                let res = storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &nm, 0, dummy_perf)
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::NodeNotBonded {
                        node_id: dummy_perf.node_id
                    }
                );

                // bonded nym-node
                let node_id = tester.bond_dummy_nymnode()?;
                let perf = liveness_submission(node_id, "0");
                let res =
                    storage.submit_performance_data(tester.deps_mut(), env.clone(), &nm, 0, perf);
                assert!(res.is_ok());

                // unbonded: the harness advances the mixnet epoch to settle the unbonding, so
                // submit for the epoch that is now current
                tester.unbond_nymnode(node_id)?;
                let epoch_id = tester.current_mixnet_epoch()?;

                let res = storage
                    .submit_performance_data(tester.deps_mut(), env.clone(), &nm, epoch_id, perf)
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::NodeNotBonded { node_id });

                Ok(())
            }
        }

        #[cfg(test)]
        mod batch_submitting_performance_data {
            use super::*;
            use nym_mixnet_contract_common::EpochState;

            #[test]
            fn rejects_an_empty_entry_even_for_a_node_that_is_not_bonded() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm = tester.new_authorised_network_monitor();
                let env = tester.env();

                // a node that is not bonded is normally skipped, but an empty payload is a
                // malformed batch and must not be skipped along with it
                let empty = NodeSubmission {
                    node_id: 999999,
                    measurements: Measurements::default(),
                };
                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        0,
                        vec![empty],
                    )
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::EmptyNodeSubmission { node_id: 999999 }
                );

                // the same holds for an entry later in the batch
                let data = tester.dummy_node_submission();
                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        0,
                        vec![data, empty],
                    )
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::EmptyNodeSubmission { node_id: 999999 }
                );

                let metadata = tester.submission_metadata(&nm);
                assert_eq!(metadata.last_submitted_node_id, 0);

                Ok(())
            }

            #[test]
            fn is_only_allowed_by_authorised_network_monitors() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm1 = tester.addr_make("network-monitor-1");
                let nm2 = tester.addr_make("network-monitor-2");
                let unauthorised = tester.addr_make("unauthorised");
                let env = tester.env();

                tester.authorise_network_monitor(&nm1);

                let perf = tester.dummy_node_submission();
                // authorised network monitor can submit the results just fine
                assert!(storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm1,
                        0,
                        vec![perf]
                    )
                    .is_ok());

                // unauthorised address is rejected
                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm2,
                        0,
                        vec![perf],
                    )
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::NotAuthorised {
                        address: nm2.clone()
                    }
                );

                // it is fine after explicit authorisation though
                tester.authorise_network_monitor(&nm2);
                assert!(storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm2,
                        0,
                        vec![perf]
                    )
                    .is_ok());

                // and address that was never authorised still fails
                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &unauthorised,
                        0,
                        vec![perf],
                    )
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::NotAuthorised {
                        address: unauthorised
                    }
                );
                Ok(())
            }

            #[test]
            fn requires_sorted_list_of_performances() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm = tester.new_authorised_network_monitor();
                let env = tester.env();

                let id1 = tester.bond_dummy_nymnode()?;
                let id2 = tester.bond_dummy_nymnode()?;
                let id3 = tester.bond_dummy_nymnode()?;
                let data = liveness_submission(id1, "1");
                let another_data = liveness_submission(id2, "1");
                let more_data = liveness_submission(id3, "1");

                let duplicates = vec![data, data];
                let another_dups = vec![another_data, another_data];
                let unsorted = vec![another_data, data];
                let semi_sorted = vec![data, more_data, another_data];
                let sorted = vec![data, another_data, more_data];

                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        0,
                        duplicates,
                    )
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::UnsortedBatchSubmission);

                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        0,
                        another_dups,
                    )
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::UnsortedBatchSubmission);

                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        0,
                        unsorted,
                    )
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::UnsortedBatchSubmission);

                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        0,
                        semi_sorted,
                    )
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::UnsortedBatchSubmission);

                assert!(storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        0,
                        sorted
                    )
                    .is_ok());
                Ok(())
            }

            #[test]
            fn its_not_possible_to_submit_data_for_same_node_again() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm = tester.new_authorised_network_monitor();
                let env = tester.env();

                let id1 = tester.bond_dummy_nymnode()?;
                let id2 = tester.bond_dummy_nymnode()?;
                let data = liveness_submission(id1, "1");
                let another_data = liveness_submission(id2, "1");

                // first submission
                assert!(storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        0,
                        vec![data]
                    )
                    .is_ok());

                // second submission
                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        0,
                        vec![data],
                    )
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::StalePerformanceSubmission {
                        epoch_id: 0,
                        node_id: id1,
                        last_epoch_id: 0,
                        last_node_id: id1,
                    }
                );

                // another submission works fine
                assert!(storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        0,
                        vec![another_data]
                    )
                    .is_ok());

                // original one works IF it's for the next epoch, once that epoch is current
                tester.set_mixnet_epoch(1)?;
                assert!(storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        1,
                        vec![data]
                    )
                    .is_ok());

                // the past epoch is now rejected by the epoch check, before the cursor is consulted
                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        0,
                        vec![data],
                    )
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::EpochNotCurrent {
                        epoch_id: 0,
                        current_epoch_id: 1,
                    }
                );

                Ok(())
            }

            #[test]
            fn its_not_possible_to_submit_data_out_of_order() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let env = tester.env();
                let nm = tester.new_authorised_network_monitor();

                let id1 = tester.bond_dummy_nymnode()?;
                let id2 = tester.bond_dummy_nymnode()?;
                let data = liveness_submission(id1, "1");
                let another_data = liveness_submission(id2, "1");

                assert!(storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        0,
                        vec![another_data]
                    )
                    .is_ok());

                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        0,
                        vec![data],
                    )
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::StalePerformanceSubmission {
                        epoch_id: 0,
                        node_id: id1,
                        last_epoch_id: 0,
                        last_node_id: id2,
                    }
                );

                // check across epochs: a new epoch resets the node ordering
                tester.set_mixnet_epoch(10)?;
                assert!(storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        10,
                        vec![data]
                    )
                    .is_ok());

                // and an earlier epoch is rejected before the cursor is consulted
                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        9,
                        vec![data],
                    )
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::EpochNotCurrent {
                        epoch_id: 9,
                        current_epoch_id: 10,
                    }
                );
                Ok(())
            }

            #[test]
            fn its_only_possible_to_submit_data_for_the_current_epoch() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let env = tester.env();

                tester.set_mixnet_epoch(10)?;
                let nm = tester.new_authorised_network_monitor();
                let data = tester.dummy_node_submission();

                // past and future epochs alike are rejected before the cursor is consulted
                for other in [0, 9, 11] {
                    let res = storage
                        .batch_submit_performance_results(
                            tester.deps_mut(),
                            env.clone(),
                            &nm,
                            other,
                            vec![data],
                        )
                        .unwrap_err();
                    assert_eq!(
                        res,
                        NymPerformanceContractError::EpochNotCurrent {
                            epoch_id: other,
                            current_epoch_id: 10,
                        }
                    );
                }

                storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    10,
                    vec![data],
                )?;

                tester.set_mixnet_epoch(11)?;
                storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    env,
                    &nm,
                    11,
                    vec![data],
                )?;

                Ok(())
            }

            #[test]
            fn its_not_possible_to_submit_data_during_the_epoch_transition() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let env = tester.env();

                let nm = tester.new_authorised_network_monitor();
                let data = tester.dummy_node_submission();

                tester.set_mixnet_epoch_status(EpochState::Rewarding {
                    last_rewarded: 0,
                    final_node_id: 42,
                });
                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        0,
                        vec![data],
                    )
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::EpochInTransition { epoch_id: 0 }
                );

                // the epoch check precedes the empty-batch short-circuit
                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        env.clone(),
                        &nm,
                        0,
                        vec![],
                    )
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::EpochInTransition { epoch_id: 0 }
                );

                tester.set_mixnet_epoch_status(EpochState::InProgress);
                storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    env,
                    &nm,
                    0,
                    vec![data],
                )?;

                Ok(())
            }

            #[test]
            fn updates_submission_metadata() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let env = tester.env();

                let nm = tester.new_authorised_network_monitor();
                let metadata = tester.submission_metadata(&nm);
                assert_eq!(metadata.last_submitted_epoch_id, 0);
                assert_eq!(metadata.last_submitted_node_id, 0);

                let nodes = tester.bond_dummy_nymnodes(10);

                // single submission
                storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    0,
                    vec![liveness_submission(nodes[0], "0")],
                )?;
                let metadata = tester.submission_metadata(&nm);
                assert_eq!(metadata.last_submitted_epoch_id, 0);
                assert_eq!(metadata.last_submitted_node_id, nodes[0]);

                // another epoch, once it is current
                tester.set_mixnet_epoch(1)?;
                storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    1,
                    vec![liveness_submission(nodes[1], "0")],
                )?;
                let metadata = tester.submission_metadata(&nm);
                assert_eq!(metadata.last_submitted_epoch_id, 1);
                assert_eq!(metadata.last_submitted_node_id, nodes[1]);

                // multiple submissions
                storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    1,
                    vec![
                        liveness_submission(nodes[2], "0"),
                        liveness_submission(nodes[3], "0"),
                        liveness_submission(nodes[4], "0"),
                    ],
                )?;
                let metadata = tester.submission_metadata(&nm);
                assert_eq!(metadata.last_submitted_epoch_id, 1);
                assert_eq!(metadata.last_submitted_node_id, nodes[4]);

                // another epoch
                tester.set_mixnet_epoch(2)?;
                storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    2,
                    vec![
                        liveness_submission(nodes[1], "0"),
                        liveness_submission(nodes[6], "0"),
                        liveness_submission(nodes[8], "0"),
                    ],
                )?;
                let metadata = tester.submission_metadata(&nm);
                assert_eq!(metadata.last_submitted_epoch_id, 2);
                assert_eq!(metadata.last_submitted_node_id, nodes[8]);

                Ok(())
            }

            #[test]
            fn updates_latest_submitted_information() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let env = tester.env();

                let nm = tester.new_authorised_network_monitor();

                let nodes = tester.bond_dummy_nymnodes(10);

                let expected = |epoch_id: EpochId, data: NodeSubmission| LastSubmission {
                    block_height: env.block.height,
                    block_time: env.block.time,
                    data: Some(LastSubmittedData {
                        sender: nm.clone(),
                        epoch_id,
                        data,
                    }),
                };

                // single submission
                let data = liveness_submission(nodes[0], "0");
                storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    0,
                    vec![data],
                )?;
                assert_eq!(
                    storage.last_performance_submission.load(&tester)?,
                    expected(0, data)
                );

                // another epoch, once it is current
                tester.set_mixnet_epoch(1)?;
                let data = liveness_submission(nodes[1], "0");
                storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    1,
                    vec![data],
                )?;
                assert_eq!(
                    storage.last_performance_submission.load(&tester)?,
                    expected(1, data)
                );

                // multiple submissions: the last entry is recorded
                let last = liveness_submission(nodes[4], "0");
                storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    1,
                    vec![
                        liveness_submission(nodes[2], "0"),
                        liveness_submission(nodes[3], "0"),
                        last,
                    ],
                )?;
                assert_eq!(
                    storage.last_performance_submission.load(&tester)?,
                    expected(1, last)
                );

                // another epoch
                tester.set_mixnet_epoch(2)?;
                let last = liveness_submission(nodes[8], "0");
                storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    2,
                    vec![
                        liveness_submission(nodes[1], "0"),
                        liveness_submission(nodes[7], "0"),
                        last,
                    ],
                )?;
                assert_eq!(
                    storage.last_performance_submission.load(&tester)?,
                    expected(2, last)
                );

                Ok(())
            }

            #[test]
            fn an_empty_batch_is_a_noop() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let env = tester.env();

                let nm = tester.new_authorised_network_monitor();

                // move the cursor and the last submission off their initial values first
                let data = tester.dummy_node_submission();
                storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    0,
                    vec![data],
                )?;
                let cursor_before = tester.submission_metadata(&nm);
                let last_before = storage.last_performance_submission.load(&tester)?;

                // an empty batch is accepted (it never reaches the cursor check) and changes nothing
                let res = storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    env,
                    &nm,
                    0,
                    vec![],
                )?;
                assert_eq!(res, BatchSubmissionResult::default());
                assert_eq!(tester.submission_metadata(&nm), cursor_before);
                assert_eq!(
                    storage.last_performance_submission.load(&tester)?,
                    last_before
                );

                Ok(())
            }

            #[test]
            fn informs_if_associated_node_is_not_bonded() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let nm = tester.new_authorised_network_monitor();

                // bond and unbond some nodes to advance the id counter
                for _ in 0..10 {
                    let node_id = tester.bond_dummy_nymnode()?;
                    tester.unbond_nymnode(node_id)?;
                }

                let nym_node1 = tester.bond_dummy_nymnode()?;
                let nym_node_between = tester.bond_dummy_nymnode()?;
                tester.unbond_nymnode(nym_node_between)?;
                let nym_node2 = tester.bond_dummy_nymnode()?;

                // every unbond above advanced the mixnet epoch, so submit for the current one
                let epoch_id = tester.current_mixnet_epoch()?;
                let env = tester.env();

                // single id - nothing bonded
                let res = storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    epoch_id,
                    vec![liveness_submission(999999, "0")],
                )?;
                assert_eq!(res.accepted_scores, 0);
                assert_eq!(res.non_existent_nodes, vec![999999]);

                // one bonded nym-node, one not bonded
                tester.set_mixnet_epoch(epoch_id + 1)?;
                let res = storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    epoch_id + 1,
                    vec![
                        liveness_submission(nym_node1, "0"),
                        liveness_submission(999999, "0"),
                    ],
                )?;
                assert_eq!(res.accepted_scores, 1);
                assert_eq!(res.non_existent_nodes, vec![999999]);

                // not-bonded, bonded, not-bonded, bonded
                tester.set_mixnet_epoch(epoch_id + 2)?;
                let res = storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    env.clone(),
                    &nm,
                    epoch_id + 2,
                    vec![
                        liveness_submission(2, "0"),
                        liveness_submission(nym_node1, "0"),
                        liveness_submission(nym_node_between, "0"),
                        liveness_submission(nym_node2, "0"),
                    ],
                )?;
                assert_eq!(res.accepted_scores, 2);
                assert_eq!(res.non_existent_nodes, vec![2, nym_node_between]);

                Ok(())
            }
        }

        #[test]
        fn checking_for_admin() -> anyhow::Result<()> {
            let mut pre_init = PreInitContract::new();
            let env = pre_init.env();
            let admin = pre_init.api.addr_make("admin");
            let non_admin = pre_init.api.addr_make("non-admin");
            let mixnet_contract = pre_init.mixnet_contract_address.clone();

            let storage = NymPerformanceContractStorage::new();

            let deps = pre_init.deps_mut();
            storage.initialise(
                deps,
                env,
                admin.clone(),
                mixnet_contract,
                Vec::new(),
                crate::testing::liveness_only_weights(),
            )?;

            let deps = pre_init.deps();
            assert!(storage.is_admin(deps, &admin)?);
            assert!(!storage.is_admin(deps, &non_admin)?);

            Ok(())
        }

        #[test]
        fn ensuring_admin_privileges() -> anyhow::Result<()> {
            let storage = NymPerformanceContractStorage::new();
            let mut pre_init = PreInitContract::new();
            let env = pre_init.env();

            let admin = pre_init.api.addr_make("admin");
            let non_admin = pre_init.api.addr_make("non-admin");
            let mixnet_contract = pre_init.mixnet_contract_address.clone();

            let deps = pre_init.deps_mut();
            storage.initialise(
                deps,
                env,
                admin.clone(),
                mixnet_contract,
                Vec::new(),
                crate::testing::liveness_only_weights(),
            )?;

            let deps = pre_init.deps();
            assert!(storage.ensure_is_admin(deps, &admin).is_ok());
            assert!(storage.ensure_is_admin(deps, &non_admin).is_err());

            Ok(())
        }

        #[cfg(test)]
        mod authorising_network_monitor {
            use super::*;
            use cw_controllers::AdminError::NotAdmin;
            use nym_contracts_common_testing::AdminExt;

            #[test]
            fn can_only_be_performed_by_contract_admin() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let admin = tester.admin_unchecked();
                let not_admin = tester.addr_make("not-admin");
                let nm = tester.addr_make("network-monitor");
                let env = tester.env();

                let res = storage
                    .authorise_network_monitor(tester.deps_mut(), &env, &not_admin, nm.clone())
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::Admin(NotAdmin {}));

                assert!(storage
                    .authorise_network_monitor(tester.deps_mut(), &env, &admin, nm)
                    .is_ok());

                // change admin
                let new_admin = tester.addr_make("new-admin");
                tester.update_admin(&Some(new_admin.clone()))?;

                let another_nm = tester.addr_make("another-network-monitor");

                // old one no longer works
                let res = storage
                    .authorise_network_monitor(tester.deps_mut(), &env, &admin, another_nm.clone())
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::Admin(NotAdmin {}));

                assert!(storage
                    .authorise_network_monitor(tester.deps_mut(), &env, &new_admin, another_nm)
                    .is_ok());
                Ok(())
            }

            #[test]
            fn network_monitor_must_not_already_be_authorised() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let admin = tester.admin_unchecked();
                let nm = tester.addr_make("network-monitor");
                let env = tester.env();

                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm.clone())?;

                let res = storage
                    .authorise_network_monitor(tester.deps_mut(), &env, &admin, nm.clone())
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::AlreadyAuthorised { address: nm }
                );

                Ok(())
            }

            #[test]
            fn for_valid_network_monitor_storage_is_updated() -> anyhow::Result<()> {
                // note: detailed invariants are checked in network_monitors_storage
                // here we just want to ensure **something** happens (i.e. `insert_new` is called)
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let admin = tester.admin_unchecked();
                let nm = tester.addr_make("network-monitor");
                let env = tester.env();

                let current_authorised = storage.network_monitors.authorised_count.load(&tester)?;
                assert_eq!(current_authorised, 0);

                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm.clone())?;

                let current_authorised = storage.network_monitors.authorised_count.load(&tester)?;
                assert_eq!(current_authorised, 1);

                Ok(())
            }

            #[test]
            fn initial_metadata_uses_current_mixnet_epoch() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let admin = tester.admin_unchecked();
                let nm1 = tester.addr_make("network-monitor1");
                let nm2 = tester.addr_make("network-monitor2");
                let nm3 = tester.addr_make("network-monitor3");
                let env = tester.env();

                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm1.clone())?;
                assert_eq!(0, tester.submission_metadata(&nm1).last_submitted_epoch_id);

                tester.advance_mixnet_epoch()?;
                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm2.clone())?;
                assert_eq!(1, tester.submission_metadata(&nm2).last_submitted_epoch_id);

                tester.set_mixnet_epoch(1000)?;
                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm3.clone())?;
                assert_eq!(
                    1000,
                    tester.submission_metadata(&nm3).last_submitted_epoch_id
                );

                Ok(())
            }
        }

        #[cfg(test)]
        mod retiring_network_monitor {
            use super::*;
            use cw_controllers::AdminError::NotAdmin;
            use nym_contracts_common_testing::AdminExt;

            #[test]
            fn can_only_be_performed_by_contract_admin() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let admin = tester.admin_unchecked();
                let not_admin = tester.addr_make("not-admin");
                let nm = tester.addr_make("network-monitor");
                let another_nm = tester.addr_make("another-network-monitor");
                let env = tester.env();

                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm.clone())?;
                storage.authorise_network_monitor(
                    tester.deps_mut(),
                    &env,
                    &admin,
                    another_nm.clone(),
                )?;

                let res = storage
                    .retire_network_monitor(tester.deps_mut(), env.clone(), &not_admin, nm.clone())
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::Admin(NotAdmin {}));

                assert!(storage
                    .retire_network_monitor(tester.deps_mut(), env.clone(), &admin, nm)
                    .is_ok());

                // change admin
                let new_admin = tester.addr_make("new-admin");
                tester.update_admin(&Some(new_admin.clone()))?;

                // old one no longer works
                let res = storage
                    .retire_network_monitor(
                        tester.deps_mut(),
                        env.clone(),
                        &admin,
                        another_nm.clone(),
                    )
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::Admin(NotAdmin {}));

                assert!(storage
                    .retire_network_monitor(tester.deps_mut(), env, &new_admin, another_nm)
                    .is_ok());

                Ok(())
            }

            #[test]
            fn for_valid_network_monitor_storage_is_updated() -> anyhow::Result<()> {
                // note: detailed invariants are checked in network_monitors_storage
                // here we just want to ensure **something** happens (i.e. `retire` is called)
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let admin = tester.admin_unchecked();
                let nm = tester.addr_make("network-monitor");
                let env = tester.env();

                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm.clone())?;

                let current_authorised = storage.network_monitors.authorised_count.load(&tester)?;
                assert_eq!(current_authorised, 1);

                storage.retire_network_monitor(tester.deps_mut(), env, &admin, nm)?;

                let current_authorised = storage.network_monitors.authorised_count.load(&tester)?;
                assert_eq!(current_authorised, 0);

                Ok(())
            }
        }

        #[cfg(test)]
        mod rewarding_inputs {
            use super::*;
            use crate::testing::liveness_only_weights;
            use nym_performance_contract_common::KindMedians;

            fn medians(liveness: &str, stress: Option<&str>, config: &str) -> KindMedians {
                KindMedians {
                    liveness: Some(p(liveness)),
                    stress: stress.map(p),
                    config: Some(p(config)),
                }
            }

            /// A bundle with liveness, stress and config, for the tests where stress matters.
            fn with_stress(node_id: NodeId, liveness: &str, stress: &str) -> NodeSubmission {
                NodeSubmission {
                    node_id,
                    measurements: Measurements::default()
                        .with_liveness(p(liveness))
                        .with_stress(p(stress))
                        .with_config(p("1")),
                }
            }

            #[test]
            fn uses_the_bundle_of_the_requested_epoch_when_it_exists() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm = tester.new_authorised_network_monitor();
                let node_id = tester.bond_dummy_nymnode()?;

                tester.submit_at_epoch(&nm, 10, scored_submission(node_id, "0.8", "0.5"));

                assert_eq!(
                    storage.resolve_rewarding_inputs(&tester, 10, node_id)?,
                    RewardingInputsResponse {
                        requested_epoch_id: 10,
                        source: Some(ResolvedMedians {
                            epoch_id: 10,
                            medians: medians("0.8", None, "0.5"),
                        }),
                        weights: Some(EpochWeights {
                            effective_from: 0,
                            weights: liveness_only_weights(),
                        }),
                        score: Some(p("0.4")),
                    }
                );

                Ok(())
            }

            #[test]
            fn falls_back_to_the_newest_earlier_bundle_within_the_lookback() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm = tester.new_authorised_network_monitor();
                let node_id = tester.bond_dummy_nymnode()?;

                tester.submit_at_epoch(&nm, 8, scored_submission(node_id, "0.8", "1"));
                tester.submit_at_epoch(&nm, 12, scored_submission(node_id, "0.9", "1"));

                // inside the gap, the pointer sits above the requested epoch and the walk starts
                // just below it
                for missing in [10, 11] {
                    let res = storage.resolve_rewarding_inputs(&tester, missing, node_id)?;
                    assert_eq!(res.requested_epoch_id, missing);
                    assert_eq!(
                        res.source,
                        Some(ResolvedMedians {
                            epoch_id: 8,
                            medians: medians("0.8", None, "1"),
                        })
                    );
                    assert_eq!(res.score, Some(p("0.8")));
                }

                // past the newest bundle, the pointer short-circuits straight to it
                let res = storage.resolve_rewarding_inputs(&tester, 13, node_id)?;
                assert_eq!(res.source.map(|source| source.epoch_id), Some(12));
                assert_eq!(res.score, Some(p("0.9")));

                Ok(())
            }

            #[test]
            fn does_not_reach_past_the_lookback() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm = tester.new_authorised_network_monitor();
                let node_id = tester.bond_dummy_nymnode()?;

                tester.submit_at_epoch(&nm, 10, scored_submission(node_id, "0.8", "1"));

                let edge = 10 + MAX_FALLBACK_LOOKBACK_EPOCHS;
                let res = storage.resolve_rewarding_inputs(&tester, edge, node_id)?;
                assert_eq!(res.source.map(|source| source.epoch_id), Some(10));
                assert_eq!(res.score, Some(p("0.8")));

                let res = storage.resolve_rewarding_inputs(&tester, edge + 1, node_id)?;
                assert_eq!(res.source, None);
                assert_eq!(res.score, None);
                // the weights are still reported: they are a property of the requested epoch
                assert!(res.weights.is_some());

                Ok(())
            }

            #[test]
            fn resolves_nothing_before_the_first_bundle() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm = tester.new_authorised_network_monitor();
                let node_id = tester.bond_dummy_nymnode()?;

                // no bundle at all: no pointer, nothing to walk
                let res = storage.resolve_rewarding_inputs(&tester, 0, node_id)?;
                assert_eq!(res.source, None);
                assert_eq!(res.score, None);

                // a pointer exists but the requested epoch is 0: nothing below it to walk
                tester.submit_at_epoch(&nm, 5, scored_submission(node_id, "0.8", "1"));
                let res = storage.resolve_rewarding_inputs(&tester, 0, node_id)?;
                assert_eq!(res.source, None);

                // and between the creation epoch and the first bundle the walk finds nothing
                let res = storage.resolve_rewarding_inputs(&tester, 3, node_id)?;
                assert_eq!(res.source, None);

                Ok(())
            }

            #[test]
            fn falls_back_per_bundle_so_a_kind_that_stopped_applying_is_not_borrowed(
            ) -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let admin = tester.admin_unchecked();
                let nm = tester.new_authorised_network_monitor();
                let node_id = tester.bond_dummy_nymnode()?;

                // 70/30 weights from epoch 1 onwards
                storage.update_weights(
                    tester.deps_mut(),
                    &admin,
                    Weights {
                        liveness: p("0.7"),
                        stress: p("0.3"),
                    },
                )?;

                // a stress-measured epoch, then one in which the node no longer has stress
                tester.submit_at_epoch(&nm, 10, with_stress(node_id, "1", "0.5"));
                tester.submit_at_epoch(&nm, 12, scored_submission(node_id, "0.8", "1"));

                // epoch 10 scores both kinds: 0.7 * 1 + 0.3 * 0.5
                let res = storage.resolve_rewarding_inputs(&tester, 10, node_id)?;
                assert_eq!(res.score, Some(p("0.85")));

                // epoch 12 has no stress and is renormalised to liveness alone
                let res = storage.resolve_rewarding_inputs(&tester, 12, node_id)?;
                assert_eq!(
                    res.source,
                    Some(ResolvedMedians {
                        epoch_id: 12,
                        medians: medians("0.8", None, "1"),
                    })
                );
                assert_eq!(res.score, Some(p("0.8")));

                // and a fallback onto 12 does not reach back into 10 for the missing stress
                let res = storage.resolve_rewarding_inputs(&tester, 13, node_id)?;
                assert_eq!(res.source.map(|source| source.epoch_id), Some(12));
                assert_eq!(res.score, Some(p("0.8")));

                Ok(())
            }

            #[test]
            fn scores_with_the_weights_of_the_requested_epoch_not_the_source() -> anyhow::Result<()>
            {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let admin = tester.admin_unchecked();
                let nm = tester.new_authorised_network_monitor();
                let node_id = tester.bond_dummy_nymnode()?;

                // the only bundle, measured under the creation weights (liveness only)
                tester.submit_at_epoch(&nm, 10, with_stress(node_id, "1", "0.5"));

                // weights change to 50/50 from epoch 11
                storage.update_weights(
                    tester.deps_mut(),
                    &admin,
                    Weights {
                        liveness: p("0.5"),
                        stress: p("0.5"),
                    },
                )?;

                // at 10 the stress median does not count
                let res = storage.resolve_rewarding_inputs(&tester, 10, node_id)?;
                assert_eq!(res.weights.map(|weights| weights.effective_from), Some(0));
                assert_eq!(res.score, Some(p("1")));

                // at 12 the same bundle is scored under the weights in force at 12
                let res = storage.resolve_rewarding_inputs(&tester, 12, node_id)?;
                assert_eq!(res.source.map(|source| source.epoch_id), Some(10));
                assert_eq!(res.weights.map(|weights| weights.effective_from), Some(11));
                assert_eq!(res.score, Some(p("0.75")));

                Ok(())
            }

            #[test]
            fn a_removed_newest_bundle_yields_nothing() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let admin = tester.admin_unchecked();
                let nm = tester.new_authorised_network_monitor();
                let node_id = tester.bond_dummy_nymnode()?;

                tester.submit_at_epoch(&nm, 9, scored_submission(node_id, "0.9", "1"));
                tester.submit_at_epoch(&nm, 10, scored_submission(node_id, "1", "1"));

                // removals never touch the pointer, so it keeps naming the removed bundle
                storage.remove_epoch_measurements(tester.deps_mut(), &admin, 10)?;
                assert_eq!(
                    storage
                        .performance_results
                        .last_known_epoch
                        .load(&tester, node_id)?,
                    10
                );

                // at and after the removed epoch nothing older is consulted
                for requested in [10, 11] {
                    let res = storage.resolve_rewarding_inputs(&tester, requested, node_id)?;
                    assert_eq!(res.source, None);
                    assert_eq!(res.score, None);
                }

                // the surviving bundle is still served for its own epoch
                let res = storage.resolve_rewarding_inputs(&tester, 9, node_id)?;
                assert_eq!(res.source.map(|source| source.epoch_id), Some(9));
                assert_eq!(res.score, Some(p("0.9")));

                Ok(())
            }

            #[test]
            fn repeated_calls_agree() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm = tester.new_authorised_network_monitor();
                let node_id = tester.bond_dummy_nymnode()?;

                tester.submit_at_epoch(&nm, 8, scored_submission(node_id, "0.8", "1"));
                let first = storage.resolve_rewarding_inputs(&tester, 10, node_id)?;
                assert_eq!(first.source.as_ref().map(|source| source.epoch_id), Some(8));

                // the mixnet moves on and more data lands, moving the pointer past the request
                tester.submit_at_epoch(&nm, 12, scored_submission(node_id, "0.1", "1"));
                tester.submit_at_epoch(&nm, 13, scored_submission(node_id, "0.2", "1"));

                assert_eq!(
                    storage.resolve_rewarding_inputs(&tester, 10, node_id)?,
                    first
                );

                Ok(())
            }
        }

        #[cfg(test)]
        mod weights {
            use super::*;
            use crate::testing::liveness_only_weights;
            use cosmwasm_std::{Decimal, StdResult};
            use cw_controllers::AdminError::NotAdmin;
            use nym_contracts_common_testing::ArbitraryContractStorageWriter;

            fn weights(liveness: &str, stress: &str) -> Weights {
                Weights {
                    liveness: p(liveness),
                    stress: p(stress),
                }
            }

            fn all_weights(
                storage: &NymPerformanceContractStorage,
                tester: &dyn Storage,
            ) -> anyhow::Result<Vec<(EpochId, Weights)>> {
                Ok(storage
                    .weights
                    .range(tester, None, None, Order::Ascending)
                    .collect::<StdResult<Vec<_>>>()?)
            }

            #[test]
            fn an_update_takes_effect_from_the_next_epoch() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let admin = tester.admin_unchecked();
                tester.set_mixnet_epoch(10)?;

                let effective_from =
                    storage.update_weights(tester.deps_mut(), &admin, weights("0.7", "0.3"))?;
                assert_eq!(effective_from, 11);

                // the running epoch keeps the weights it started with
                assert_eq!(
                    storage.weights_at(&tester, 10)?,
                    Some(EpochWeights {
                        effective_from: 0,
                        weights: liveness_only_weights(),
                    })
                );

                // and every epoch from the next one onwards resolves to the update
                let updated = EpochWeights {
                    effective_from: 11,
                    weights: weights("0.7", "0.3"),
                };
                assert_eq!(storage.weights_at(&tester, 11)?, Some(updated.clone()));
                assert_eq!(storage.weights_at(&tester, 500)?, Some(updated));

                Ok(())
            }

            #[test]
            fn two_updates_in_one_epoch_overwrite() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let admin = tester.admin_unchecked();
                tester.set_mixnet_epoch(10)?;

                storage.update_weights(tester.deps_mut(), &admin, weights("0.6", "0.4"))?;
                storage.update_weights(tester.deps_mut(), &admin, weights("0.5", "0.5"))?;

                assert_eq!(
                    all_weights(&storage, &tester)?,
                    vec![(0, liveness_only_weights()), (11, weights("0.5", "0.5"))]
                );

                Ok(())
            }

            #[test]
            fn can_only_be_performed_by_contract_admin() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let not_admin = tester.addr_make("not-admin");

                let res = storage
                    .update_weights(tester.deps_mut(), &not_admin, weights("0.7", "0.3"))
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::Admin(NotAdmin {}));
                assert_eq!(
                    all_weights(&storage, &tester)?,
                    vec![(0, liveness_only_weights())]
                );

                Ok(())
            }

            #[test]
            fn rejects_invalid_weights_without_storing_them() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let admin = tester.admin_unchecked();

                let res = storage
                    .update_weights(tester.deps_mut(), &admin, weights("0.7", "0.2"))
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::WeightsDoNotSumToOne {
                        total: Decimal::percent(90)
                    }
                );

                let res = storage
                    .update_weights(tester.deps_mut(), &admin, weights("0", "0"))
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::EmptyWeights);

                assert_eq!(
                    all_weights(&storage, &tester)?,
                    vec![(0, liveness_only_weights())]
                );

                Ok(())
            }

            #[test]
            fn there_are_no_weights_before_the_creation_epoch() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut pre_init = PreInitContract::new();
                let address = pre_init.mixnet_contract_address.clone();

                let mut interval = pre_init.querier().query_current_mixnet_interval(&address)?;
                for _ in 0..5 {
                    interval = interval.advance_epoch();
                }
                pre_init.set_contract_storage_value(&address, b"ci", &interval)?;

                let env = pre_init.env();
                let admin = pre_init.addr_make("admin");
                storage.initialise(
                    pre_init.deps_mut(),
                    env,
                    admin,
                    address,
                    Vec::new(),
                    liveness_only_weights(),
                )?;

                let deps = pre_init.deps();
                assert_eq!(storage.weights_at(deps.storage, 4)?, None);
                assert_eq!(
                    storage.weights_at(deps.storage, 5)?,
                    Some(EpochWeights {
                        effective_from: 5,
                        weights: liveness_only_weights(),
                    })
                );

                Ok(())
            }
        }

        #[test]
        fn loading_performance_data() -> anyhow::Result<()> {
            let storage = NymPerformanceContractStorage::new();
            let mut tester = init_contract_tester();
            let admin = tester.admin_unchecked();
            let nms: Vec<Addr> = (0..6)
                .map(|_| tester.new_authorised_network_monitor())
                .collect();

            // with every monitor reporting config 100%, the score under the default
            // `Liveness: 100%` weights is the liveness median itself
            let expected = |liveness: &str| EpochNodePerformance {
                epoch_id: 0,
                medians: KindMedians {
                    liveness: Some(p(liveness)),
                    stress: None,
                    config: Some(p("1")),
                },
                score: Some(p(liveness)),
            };

            // no results
            let node_id = tester.bond_dummy_nymnode()?;
            assert_eq!(storage.try_load_performance(&tester, 0, node_id)?, None);

            //
            // always returns median value with 2decimal places precision
            //

            // single result
            let node_id = tester.bond_dummy_nymnode()?;
            tester.submit_scored(&nms[0], node_id, "0.42");
            assert_eq!(
                storage.try_load_performance(&tester, 0, node_id)?,
                Some(expected("0.42"))
            );

            // two results (median doesn't require changing decimal places)
            let node_id = tester.bond_dummy_nymnode()?;
            tester.submit_scored(&nms[0], node_id, "0.50");
            tester.submit_scored(&nms[1], node_id, "0.40");
            assert_eq!(
                storage.try_load_performance(&tester, 0, node_id)?,
                Some(expected("0.45"))
            );

            // two results (median requires changing decimal places)
            let node_id = tester.bond_dummy_nymnode()?;
            tester.submit_scored(&nms[0], node_id, "0.58");
            tester.submit_scored(&nms[1], node_id, "0.45");
            assert_eq!(
                storage.try_load_performance(&tester, 0, node_id)?,
                Some(expected("0.52"))
            );

            // three results (median is the middle value rather than the average)
            let node_id = tester.bond_dummy_nymnode()?;
            tester.submit_scored(&nms[0], node_id, "0.12");
            tester.submit_scored(&nms[1], node_id, "0.34");
            tester.submit_scored(&nms[2], node_id, "0.56");
            assert_eq!(
                storage.try_load_performance(&tester, 0, node_id)?,
                Some(expected("0.34"))
            );

            // five results (notice how they're not inserted sorted)
            let node_id = tester.bond_dummy_nymnode()?;
            tester.submit_scored(&nms[0], node_id, "0.9");
            tester.submit_scored(&nms[1], node_id, "0.9");
            tester.submit_scored(&nms[2], node_id, "0.1");
            tester.submit_scored(&nms[4], node_id, "0.1");
            tester.submit_scored(&nms[5], node_id, "0.7");
            assert_eq!(
                storage.try_load_performance(&tester, 0, node_id)?,
                Some(expected("0.7"))
            );

            // six results (same as above, but average of middle values)
            let node_id = tester.bond_dummy_nymnode()?;
            tester.submit_scored(&nms[0], node_id, "0.9");
            tester.submit_scored(&nms[1], node_id, "0.9");
            tester.submit_scored(&nms[2], node_id, "0.1");
            tester.submit_scored(&nms[3], node_id, "0.1");
            tester.submit_scored(&nms[4], node_id, "0.2");
            tester.submit_scored(&nms[5], node_id, "0.3");
            assert_eq!(
                storage.try_load_performance(&tester, 0, node_id)?,
                Some(expected("0.25"))
            );

            // the config median gates the score: liveness 0.8 under configs 1 and 0.5
            let node_id = tester.bond_dummy_nymnode()?;
            tester.submit_scored(&nms[0], node_id, "0.8");
            tester.submit_now(&nms[1], scored_submission(node_id, "0.8", "0.5"));
            assert_eq!(
                storage.try_load_performance(&tester, 0, node_id)?,
                Some(EpochNodePerformance {
                    epoch_id: 0,
                    medians: KindMedians {
                        liveness: Some(p("0.8")),
                        stress: None,
                        config: Some(p("0.75")),
                    },
                    score: Some(p("0.6")),
                })
            );

            // a bundle without config has medians but no score
            let node_id = tester.bond_dummy_nymnode()?;
            tester.submit_liveness(&nms[0], node_id, "0.42");
            assert_eq!(
                storage.try_load_performance(&tester, 0, node_id)?,
                Some(EpochNodePerformance {
                    epoch_id: 0,
                    medians: KindMedians {
                        liveness: Some(p("0.42")),
                        stress: None,
                        config: None,
                    },
                    score: None,
                })
            );

            // and the score follows the weights of the bundle's own epoch
            storage.update_weights(
                tester.deps_mut(),
                &admin,
                Weights {
                    liveness: p("0.5"),
                    stress: p("0.5"),
                },
            )?;
            tester.set_mixnet_epoch(1)?;
            let node_id = tester.bond_dummy_nymnode()?;
            tester.submit_now(
                &nms[0],
                NodeSubmission {
                    node_id,
                    measurements: Measurements::default()
                        .with_liveness(p("1"))
                        .with_stress(p("0.5"))
                        .with_config(p("1")),
                },
            );
            let perf = storage.try_load_performance(&tester, 1, node_id)?;
            assert_eq!(perf.as_ref().map(|perf| perf.epoch_id), Some(1));
            assert_eq!(perf.and_then(|perf| perf.score), Some(p("0.75")));

            Ok(())
        }

        #[cfg(test)]
        mod removing_node_measurements {
            use super::*;
            use crate::testing::liveness_only_weights;
            use cw_controllers::AdminError::NotAdmin;
            use nym_contracts_common_testing::FullReader;

            #[test]
            fn can_only_be_performed_by_contract_admin() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let admin = tester.admin_unchecked();
                let not_admin = tester.addr_make("not-admin");
                let nm = tester.new_authorised_network_monitor();

                let epoch_id = 0;
                let id1 = tester.bond_dummy_nymnode()?;
                let id2 = tester.bond_dummy_nymnode()?;

                tester.submit_liveness(&nm, id1, "0.42");
                tester.submit_liveness(&nm, id2, "0.42");

                let res = storage
                    .remove_node_measurements(tester.deps_mut(), &not_admin, epoch_id, id1)
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::Admin(NotAdmin {}));

                assert!(storage
                    .remove_node_measurements(tester.deps_mut(), &admin, epoch_id, id1)
                    .is_ok());

                // change admin
                let new_admin = tester.addr_make("new-admin");
                tester.update_admin(&Some(new_admin.clone()))?;

                // old one no longer works
                let res = storage
                    .remove_node_measurements(tester.deps_mut(), &admin, epoch_id, id2)
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::Admin(NotAdmin {}));

                assert!(storage
                    .remove_node_measurements(tester.deps_mut(), &new_admin, epoch_id, id2)
                    .is_ok());

                Ok(())
            }

            #[test]
            fn is_noop_if_entry_didnt_exist() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let admin = tester.admin_unchecked();
                let epoch_id = 0;
                let node_id = 0;

                let before = storage.performance_results.results.all_values(&tester)?;
                assert!(before.is_empty());

                storage.remove_node_measurements(tester.deps_mut(), &admin, epoch_id, node_id)?;

                let after = storage.performance_results.results.all_values(&tester)?;
                assert!(after.is_empty());

                Ok(())
            }

            #[test]
            fn removes_the_underlying_data() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let admin = tester.admin_unchecked();
                let nm1 = tester.new_authorised_network_monitor();
                let nm2 = tester.new_authorised_network_monitor();
                let nm3 = tester.new_authorised_network_monitor();

                let id1 = tester.bond_dummy_nymnode()?;
                let id2 = tester.bond_dummy_nymnode()?;

                let epoch_id = 0;

                // single measurement
                tester.submit_liveness(&nm1, id1, "0.42");

                let before = storage
                    .performance_results
                    .results
                    .may_load(&tester, (epoch_id, id1))?;
                assert!(before.is_some());

                storage.remove_node_measurements(tester.deps_mut(), &admin, epoch_id, id1)?;

                let after = storage
                    .performance_results
                    .results
                    .may_load(&tester, (epoch_id, id1))?;
                assert!(after.is_none());

                // the removal leaves the node's last-known epoch and the weights alone
                assert_eq!(tester.last_known_epoch(id1), Some(epoch_id));
                assert_eq!(
                    storage
                        .weights_at(&tester, epoch_id)?
                        .map(|weights| weights.weights),
                    Some(liveness_only_weights())
                );

                // multiple measurements
                tester.submit_liveness(&nm1, id2, "0.42");
                tester.submit_liveness(&nm2, id2, "0.69");
                tester.submit_liveness(&nm3, id2, "1");

                let before = storage
                    .performance_results
                    .results
                    .may_load(&tester, (epoch_id, id2))?;
                assert!(before.is_some());

                storage.remove_node_measurements(tester.deps_mut(), &admin, epoch_id, id2)?;

                let after = storage
                    .performance_results
                    .results
                    .may_load(&tester, (epoch_id, id2))?;
                assert!(after.is_none());

                Ok(())
            }
        }

        #[cfg(test)]
        mod removing_epoch_measurements {
            use super::*;
            use crate::testing::liveness_only_weights;
            use cw_controllers::AdminError::NotAdmin;
            use nym_contracts_common_testing::FullReader;

            #[test]
            fn can_only_be_performed_by_contract_admin() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let admin = tester.admin_unchecked();
                let not_admin = tester.addr_make("not-admin");
                let nm = tester.new_authorised_network_monitor();

                let id1 = tester.bond_dummy_nymnode()?;
                let id2 = tester.bond_dummy_nymnode()?;

                // epoch 0
                tester.submit_liveness(&nm, id1, "0.42");
                tester.submit_liveness(&nm, id2, "0.42");

                // epoch 1
                tester.advance_mixnet_epoch()?;
                tester.submit_liveness(&nm, id1, "0.42");
                tester.submit_liveness(&nm, id2, "0.42");

                let res = storage
                    .remove_epoch_measurements(tester.deps_mut(), &not_admin, 0)
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::Admin(NotAdmin {}));

                assert!(storage
                    .remove_epoch_measurements(tester.deps_mut(), &admin, 0)
                    .is_ok());

                // change admin
                let new_admin = tester.addr_make("new-admin");
                tester.update_admin(&Some(new_admin.clone()))?;

                // old one no longer works
                let res = storage
                    .remove_epoch_measurements(tester.deps_mut(), &admin, 1)
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::Admin(NotAdmin {}));

                assert!(storage
                    .remove_epoch_measurements(tester.deps_mut(), &new_admin, 1)
                    .is_ok());

                Ok(())
            }

            #[test]
            fn is_noop_for_empty_epochs() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let admin = tester.admin_unchecked();
                let epoch_id = 0;

                let before = storage.performance_results.results.all_values(&tester)?;
                assert!(before.is_empty());

                storage.remove_epoch_measurements(tester.deps_mut(), &admin, epoch_id)?;

                let after = storage.performance_results.results.all_values(&tester)?;
                assert!(after.is_empty());

                Ok(())
            }

            #[test]
            fn removes_the_underlying_data_below_limit() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let admin = tester.admin_unchecked();
                let nm = tester.new_authorised_network_monitor();

                // just few entries
                let epoch_id = 0;
                let nodes = tester.bond_dummy_nymnodes(10);
                for &node_id in &nodes {
                    tester.submit_liveness(&nm, node_id, "0.42");
                }

                let before = storage
                    .performance_results
                    .results
                    .prefix(epoch_id)
                    .all_values(&tester)?;
                assert_eq!(before.len(), 10);

                let res = storage.remove_epoch_measurements(tester.deps_mut(), &admin, epoch_id)?;
                assert!(!res.additional_entries_to_remove_remaining);
                let after = storage
                    .performance_results
                    .results
                    .prefix(epoch_id)
                    .all_values(&tester)?;

                assert!(after.is_empty());

                // the purge leaves every node's last-known epoch and the weights alone
                for node_id in nodes {
                    assert_eq!(tester.last_known_epoch(node_id), Some(epoch_id));
                }
                assert_eq!(
                    storage
                        .weights_at(&tester, epoch_id)?
                        .map(|weights| weights.weights),
                    Some(liveness_only_weights())
                );

                // EXACT limit
                let epoch_id = 1;
                tester.advance_mixnet_epoch()?;
                for _ in 0..retrieval_limits::EPOCH_PERFORMANCE_PURGE_LIMIT {
                    let node_id = tester.bond_dummy_nymnode()?;
                    tester.submit_liveness(&nm, node_id, "0.42");
                }

                let res = storage.remove_epoch_measurements(tester.deps_mut(), &admin, epoch_id)?;
                assert!(!res.additional_entries_to_remove_remaining);
                let after = storage
                    .performance_results
                    .results
                    .prefix(epoch_id)
                    .all_values(&tester)?;

                assert!(after.is_empty());

                Ok(())
            }

            #[test]
            fn indicates_need_for_further_calls_above_limit() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let admin = tester.admin_unchecked();
                let nm = tester.new_authorised_network_monitor();

                // just few entries
                let epoch_id = 0;
                for _ in 0..2 * retrieval_limits::EPOCH_PERFORMANCE_PURGE_LIMIT + 50 {
                    let node_id = tester.bond_dummy_nymnode()?;
                    tester.submit_liveness(&nm, node_id, "0.42");
                }

                let before = storage
                    .performance_results
                    .results
                    .prefix(epoch_id)
                    .all_values(&tester)?;
                assert_eq!(
                    before.len(),
                    2 * retrieval_limits::EPOCH_PERFORMANCE_PURGE_LIMIT + 50
                );

                let res = storage.remove_epoch_measurements(tester.deps_mut(), &admin, epoch_id)?;
                assert!(res.additional_entries_to_remove_remaining);
                let after = storage
                    .performance_results
                    .results
                    .prefix(epoch_id)
                    .all_values(&tester)?;

                assert_eq!(
                    after.len(),
                    retrieval_limits::EPOCH_PERFORMANCE_PURGE_LIMIT + 50
                );

                let res = storage.remove_epoch_measurements(tester.deps_mut(), &admin, epoch_id)?;
                assert!(res.additional_entries_to_remove_remaining);
                let after = storage
                    .performance_results
                    .results
                    .prefix(epoch_id)
                    .all_values(&tester)?;

                assert_eq!(after.len(), 50);

                let res = storage.remove_epoch_measurements(tester.deps_mut(), &admin, epoch_id)?;
                assert!(!res.additional_entries_to_remove_remaining);
                let after = storage
                    .performance_results
                    .results
                    .prefix(epoch_id)
                    .all_values(&tester)?;

                assert!(after.is_empty());

                Ok(())
            }
        }
    }

    #[cfg(test)]
    mod network_monitors_storage {
        use super::*;
        use crate::testing::{init_contract_tester, PerformanceContractTesterExt};
        use nym_contracts_common_testing::{AdminExt, ContractOpts};

        #[test]
        fn inserting_new_entry() -> anyhow::Result<()> {
            let main_storage = NymPerformanceContractStorage::new();

            let storage = NetworkMonitorsStorage::new();
            let mut tester = init_contract_tester();
            let env = tester.env();

            let admin = tester.admin_unchecked();
            let nm1 = tester.addr_make("network-monitor1");
            let nm2 = tester.addr_make("network-monitor2");

            assert!(storage
                .insert_new(tester.deps_mut(), &env, &admin, &nm1)
                .is_ok());

            // total authorised count is incremented
            assert_eq!(storage.authorised_count.load(&tester)?, 1);

            // its current data is saved
            assert_eq!(
                storage.authorised.load(&tester, &nm1)?,
                NetworkMonitorDetails {
                    address: nm1.clone(),
                    authorised_by: admin.clone(),
                    authorised_at_height: env.block.height,
                }
            );

            assert!(storage
                .insert_new(tester.deps_mut(), &env, &admin, &nm2)
                .is_ok());

            assert_eq!(storage.authorised_count.load(&tester)?, 2);
            assert_eq!(
                storage.authorised.load(&tester, &nm2)?,
                NetworkMonitorDetails {
                    address: nm2.clone(),
                    authorised_by: admin.clone(),
                    authorised_at_height: env.block.height,
                }
            );

            main_storage.retire_network_monitor(
                tester.deps_mut(),
                env.clone(),
                &admin,
                nm1.clone(),
            )?;
            assert!(storage.retired.may_load(&tester, &nm1)?.is_some());

            // if it was previously retired, that information is purged
            assert!(storage
                .insert_new(tester.deps_mut(), &env, &admin, &nm1)
                .is_ok());

            assert!(storage.retired.may_load(&tester, &nm1)?.is_none());

            Ok(())
        }

        #[test]
        fn retiring_existing_monitor() -> anyhow::Result<()> {
            let storage = NetworkMonitorsStorage::new();
            let mut tester = init_contract_tester();
            let env = tester.env();

            let admin = tester.admin_unchecked();
            let nm1 = tester.addr_make("network-monitor1");
            let nm2 = tester.addr_make("network-monitor2");
            let nm3 = tester.addr_make("network-monitor3");

            tester.authorise_network_monitor(&nm1);
            tester.authorise_network_monitor(&nm2);

            // fails on unauthorised NMs
            assert!(storage
                .retire(tester.deps_mut(), &env, &admin, &nm3)
                .is_err());

            assert_eq!(storage.authorised_count.load(&tester)?, 2);

            storage.retire(tester.deps_mut(), &env, &admin, &nm1)?;

            // total authorised count is decremented
            assert_eq!(storage.authorised_count.load(&tester)?, 1);

            // data is removed
            assert!(storage.authorised.may_load(&tester, &nm1)?.is_none());
            assert_eq!(
                storage.retired.load(&tester, &nm1)?,
                RetiredNetworkMonitor {
                    details: NetworkMonitorDetails {
                        address: nm1.clone(),
                        authorised_by: admin.clone(),
                        authorised_at_height: env.block.height,
                    },
                    retired_by: admin.clone(),
                    retired_at_height: env.block.height,
                }
            );

            storage.retire(tester.deps_mut(), &env, &admin, &nm2)?;

            assert_eq!(storage.authorised_count.load(&tester)?, 0);
            assert!(storage.authorised.may_load(&tester, &nm2)?.is_none());
            assert_eq!(
                storage.retired.load(&tester, &nm2)?,
                RetiredNetworkMonitor {
                    details: NetworkMonitorDetails {
                        address: nm2.clone(),
                        authorised_by: admin.clone(),
                        authorised_at_height: env.block.height,
                    },
                    retired_by: admin.clone(),
                    retired_at_height: env.block.height,
                }
            );

            Ok(())
        }
    }

    #[cfg(test)]
    mod performance_storage {
        use super::*;
        use crate::testing::{
            init_contract_tester, liveness_submission, p, values, PerformanceContractTesterExt,
        };
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use nym_contracts_common_testing::ContractOpts;
        use nym_performance_contract_common::Measurements;

        #[test]
        fn inserting_new_entry() -> anyhow::Result<()> {
            // essentially make sure there are no silly bugs that epoch_id and node_id got accidentally mixed up
            // when constructing map key...
            let storage = PerformanceResultsStorage::new();
            let mut tester = init_contract_tester();

            let node_id1 = 123;
            let node_id2 = 456;

            assert!(storage.results.may_load(&tester, (1, node_id1))?.is_none());
            assert!(storage.results.may_load(&tester, (1, node_id2))?.is_none());

            // a lone value creates the bundle
            storage.insert_performance_data(
                &mut tester,
                1,
                liveness_submission(node_id1, "0.23"),
            )?;
            let bundle = tester.read_bundle(1, node_id1);
            assert_eq!(values(bundle.liveness.as_ref().unwrap()), vec![p("0.23")]);
            assert!(bundle.stress.is_none());
            assert!(bundle.config.is_none());

            // a second monitor merges into it, kind by kind
            storage.insert_performance_data(
                &mut tester,
                1,
                NodeSubmission {
                    node_id: node_id1,
                    measurements: Measurements::default()
                        .with_liveness(p("1"))
                        .with_config(p("1")),
                },
            )?;
            let bundle = tester.read_bundle(1, node_id1);
            assert_eq!(
                values(bundle.liveness.as_ref().unwrap()),
                vec![p("0.23"), p("1")]
            );
            assert_eq!(values(bundle.config.as_ref().unwrap()), vec![p("1")]);
            assert!(bundle.stress.is_none());

            // values are rounded to two decimal places on the way in
            storage.insert_performance_data(
                &mut tester,
                1,
                liveness_submission(node_id2, "0.23643634"),
            )?;
            let bundle = tester.read_bundle(1, node_id2);
            assert_eq!(values(bundle.liveness.as_ref().unwrap()), vec![p("0.24")]);

            // and other epochs are separate bundles
            storage.insert_performance_data(&mut tester, 2, liveness_submission(node_id1, "1"))?;
            storage.insert_performance_data(&mut tester, 2, liveness_submission(node_id1, "1"))?;
            let bundle = tester.read_bundle(2, node_id1);
            assert_eq!(
                values(bundle.liveness.as_ref().unwrap()),
                vec![p("1"), p("1")]
            );
            assert!(storage.results.may_load(&tester, (2, node_id2))?.is_none());

            Ok(())
        }

        #[test]
        fn creating_a_bundle_advances_the_last_known_epoch() -> anyhow::Result<()> {
            let storage = PerformanceResultsStorage::new();
            let mut tester = init_contract_tester();
            let node_id = 7;

            // nothing is known before the first bundle
            assert!(storage
                .last_known_epoch
                .may_load(&tester, node_id)?
                .is_none());

            // the first monitor creates the bundle and with it the pointer
            storage.insert_performance_data(
                &mut tester,
                10,
                liveness_submission(node_id, "0.9"),
            )?;
            assert_eq!(storage.last_known_epoch.load(&tester, node_id)?, 10);

            // a later epoch moves it forward
            storage.insert_performance_data(
                &mut tester,
                11,
                liveness_submission(node_id, "0.9"),
            )?;
            assert_eq!(storage.last_known_epoch.load(&tester, node_id)?, 11);

            // but it never moves back, even when a bundle is created for an earlier epoch
            storage.insert_performance_data(&mut tester, 5, liveness_submission(node_id, "0.9"))?;
            assert_eq!(storage.last_known_epoch.load(&tester, node_id)?, 11);

            // and every node has its own
            assert!(storage.last_known_epoch.may_load(&tester, 8)?.is_none());

            Ok(())
        }

        #[test]
        fn merging_into_an_existing_bundle_leaves_the_last_known_epoch_alone() -> anyhow::Result<()>
        {
            let storage = PerformanceResultsStorage::new();
            let mut tester = init_contract_tester();
            let node_id = 7;

            storage.insert_performance_data(
                &mut tester,
                10,
                liveness_submission(node_id, "0.9"),
            )?;
            assert_eq!(storage.last_known_epoch.load(&tester, node_id)?, 10);

            // a max-write would be indistinguishable from no write, so lower the pointer by hand
            storage.last_known_epoch.save(&mut tester, node_id, &3)?;

            // the second monitor merges into the existing bundle and must not touch the pointer
            storage.insert_performance_data(
                &mut tester,
                10,
                liveness_submission(node_id, "0.8"),
            )?;
            assert_eq!(storage.last_known_epoch.load(&tester, node_id)?, 3);
            let bundle = tester.read_bundle(10, node_id);
            assert_eq!(
                values(bundle.liveness.as_ref().unwrap()),
                vec![p("0.8"), p("0.9")]
            );

            Ok(())
        }

        #[test]
        fn checking_for_submission_staleness() -> anyhow::Result<()> {
            let storage = PerformanceResultsStorage::new();
            let mut tester = init_contract_tester();

            let id1 = tester.bond_dummy_nymnode()?;
            let id2 = tester.bond_dummy_nymnode()?;
            let id3 = tester.bond_dummy_nymnode()?;

            let nm = tester.addr_make("network-monitor");
            tester.authorise_network_monitor(&nm);

            // move the cursor to (2, id2) through a real submission
            tester.set_mixnet_epoch(2)?;
            let env = tester.env();
            NymPerformanceContractStorage::new().submit_performance_data(
                tester.deps_mut(),
                env,
                &nm,
                2,
                liveness_submission(id2, "1"),
            )?;

            // illegal to submit anything < than last used epoch (unreachable through the public
            // path now that the epoch check comes first, but the cursor still guards it)
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 0, id2)
                .is_err());
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 1, id2)
                .is_err());
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 1, id3)
                .is_err());

            // for the current epoch, node id has to be greater than what has already been submitted
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 2, id1)
                .is_err());
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 2, id2)
                .is_err());
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 2, id3)
                .is_ok());

            // and anything for future epochs is fine (as long as it's the first entry)
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 3, id1)
                .is_ok());
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 3, id2)
                .is_ok());
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 1111, id3)
                .is_ok());

            Ok(())
        }
    }
}
