// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::MixnetContractQuerier;
use cosmwasm_std::{Addr, Deps, DepsMut, Env, StdError, Storage};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use nym_contracts_common::Percent;
use nym_performance_contract_common::constants::storage_keys;
use nym_performance_contract_common::{
    EpochId, NetworkMonitorDetails, NetworkMonitorSubmissionMetadata, NodeId, NodePerformance,
    NodeResults, NymPerformanceContractError, RemoveEpochMeasurementsResponse,
    RetiredNetworkMonitor,
};

pub const NYM_PERFORMANCE_CONTRACT_STORAGE: NymPerformanceContractStorage =
    NymPerformanceContractStorage::new();

pub struct NymPerformanceContractStorage {
    pub(crate) contract_admin: Admin,
    pub(crate) mixnet_epoch_id_at_creation: Item<EpochId>,

    pub(crate) mixnet_contract_address: Item<Addr>,

    pub(crate) network_monitors: NetworkMonitorsStorage,

    pub(crate) performance_results: PerformanceResultsStorage,
}

impl NymPerformanceContractStorage {
    #[allow(clippy::new_without_default)]
    pub(crate) const fn new() -> Self {
        NymPerformanceContractStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
            mixnet_epoch_id_at_creation: Item::new(storage_keys::INITIAL_EPOCH_ID),
            mixnet_contract_address: Item::new(storage_keys::MIXNET_CONTRACT),
            network_monitors: NetworkMonitorsStorage::new(),
            performance_results: PerformanceResultsStorage::new(),
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

    pub fn initialise(
        &self,
        mut deps: DepsMut,
        env: Env,
        admin: Addr,
        mixnet_contract_address: Addr,
        initial_authorised_network_monitors: Vec<String>,
    ) -> Result<(), NymPerformanceContractError> {
        // set the mixnet contract address
        self.mixnet_contract_address
            .save(deps.storage, &mixnet_contract_address)?;

        let initial_epoch_id = self.current_mixnet_epoch_id(deps.as_ref())?;

        // set the initial epoch id
        self.mixnet_epoch_id_at_creation
            .save(deps.storage, &initial_epoch_id)?;

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
        sender: &Addr,
        epoch_id: EpochId,
        data: NodePerformance,
    ) -> Result<(), NymPerformanceContractError> {
        // 1. check if the sender is authorised to submit performance data
        self.network_monitors
            .ensure_authorised(deps.storage, sender)?;

        // 2. check if current submission metadata is consistent with the result we want to submit
        self.performance_results.ensure_non_stale_submission(
            deps.storage,
            sender,
            epoch_id,
            data.node_id,
        )?;

        // 3 insert performance data into the storage
        self.performance_results
            .insert_performance_data(deps.storage, epoch_id, &data)?;

        // 4. update submission metadata based on the last result we submitted
        self.performance_results.update_submission_metadata(
            deps.storage,
            sender,
            epoch_id,
            data.node_id,
        )?;

        Ok(())
    }

    pub fn batch_submit_performance_results(
        &self,
        deps: DepsMut,
        sender: &Addr,
        epoch_id: EpochId,
        data: Vec<NodePerformance>,
    ) -> Result<(), NymPerformanceContractError> {
        // 1. check if the sender is authorised to submit performance data
        self.network_monitors
            .ensure_authorised(deps.storage, sender)?;

        let Some(first) = data.first() else {
            // no performance data
            return Ok(());
        };

        // 2. check if current submission metadata is consistent with the first result we want to submit
        self.performance_results.ensure_non_stale_submission(
            deps.storage,
            sender,
            epoch_id,
            first.node_id,
        )?;

        // 3. submit it
        self.performance_results
            .insert_performance_data(deps.storage, epoch_id, first)?;

        // not point in using peekable iterator, we can just keep track of the previous
        // element we've seen
        let mut previous = first.node_id;

        for perf in data.iter().skip(1) {
            // 4. ensure provided data is sorted (if the check fails in later iteration,
            // the whole tx will get reverted so it's fine to just set the storage within the same loop
            if perf.node_id <= previous {
                return Err(NymPerformanceContractError::UnsortedBatchSubmission);
            }
            previous = perf.node_id;

            // 5. insert performance data into the storage
            self.performance_results
                .insert_performance_data(deps.storage, epoch_id, perf)?;
        }

        // SAFETY: we know this vector is not empty
        #[allow(clippy::unwrap_used)]
        let last = data.last().unwrap();

        // 5. update submission metadata based on the last result we submitted
        self.performance_results.update_submission_metadata(
            deps.storage,
            sender,
            epoch_id,
            last.node_id,
        )?;

        Ok(())
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

    pub fn try_load_performance(
        &self,
        storage: &dyn Storage,
        epoch_id: EpochId,
        node_id: NodeId,
    ) -> Result<Option<Percent>, NymPerformanceContractError> {
        Ok(self
            .performance_results
            .results
            .may_load(storage, (epoch_id, node_id))?
            .map(|r| r.median()))
    }

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
    pub(crate) results: Map<(EpochId, NodeId), NodeResults>,

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
            submission_metadata: Map::new(storage_keys::SUBMISSION_METADATA),
        }
    }

    // note: this method assumes authorisation has been checked and invariants validated
    // (such as attempting to insert stale data)
    fn insert_performance_data(
        &self,
        storage: &mut dyn Storage,
        epoch_id: EpochId,
        data: &NodePerformance,
    ) -> Result<(), NymPerformanceContractError> {
        let performance = data.performance;

        let key = (epoch_id, data.node_id);
        let updated = match self.results.may_load(storage, key)? {
            None => NodeResults::new(performance),
            Some(mut existing) => {
                existing.insert_new(performance);
                existing
            }
        };

        self.results.save(storage, key, &updated)?;
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

    fn dummy_node_performance() -> NodePerformance {
        NodePerformance {
            node_id: 420,
            performance: Percent::from_percentage_value(69).unwrap(),
        }
    }

    #[cfg(test)]
    mod performance_contract_storage {
        use super::*;
        use crate::testing::{init_contract_tester, PerformanceContractTesterExt, PreInitContract};
        use nym_contracts_common_testing::{AdminExt, ContractOpts};

        #[cfg(test)]
        mod initialisation {
            use super::*;
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

                storage.initialise(deps, env, admin, mixnet_contract.clone(), Vec::new())?;
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

            #[test]
            fn is_only_allowed_by_authorised_network_monitors() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm1 = tester.addr_make("network-monitor-1");
                let nm2 = tester.addr_make("network-monitor-2");
                let unauthorised = tester.addr_make("unauthorised");

                tester.authorise_network_monitor(&nm1)?;

                // authorised network monitor can submit the results just fine
                assert!(storage
                    .submit_performance_data(tester.deps_mut(), &nm1, 0, dummy_node_performance())
                    .is_ok());

                // unauthorised address is rejected
                let res = storage
                    .submit_performance_data(tester.deps_mut(), &nm2, 0, dummy_node_performance())
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::NotAuthorised {
                        address: nm2.clone()
                    }
                );

                // it is fine after explicit authorisation though
                tester.authorise_network_monitor(&nm2)?;
                assert!(storage
                    .submit_performance_data(tester.deps_mut(), &nm2, 0, dummy_node_performance())
                    .is_ok());

                // and address that was never authorised still fails
                let res = storage
                    .submit_performance_data(
                        tester.deps_mut(),
                        &unauthorised,
                        0,
                        dummy_node_performance(),
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
            fn its_not_possible_to_submit_data_for_same_node_again() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm = tester.addr_make("network-monitor");
                tester.authorise_network_monitor(&nm)?;

                let data = NodePerformance {
                    node_id: 123,
                    performance: Percent::hundred(),
                };
                let another_data = NodePerformance {
                    node_id: 124,
                    performance: Percent::hundred(),
                };

                // first submission
                assert!(storage
                    .submit_performance_data(tester.deps_mut(), &nm, 0, data)
                    .is_ok());

                // second submission
                let res = storage
                    .submit_performance_data(tester.deps_mut(), &nm, 0, data)
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::StalePerformanceSubmission {
                        epoch_id: 0,
                        node_id: 123,
                        last_epoch_id: 0,
                        last_node_id: 123,
                    }
                );

                // another submission works fine
                assert!(storage
                    .submit_performance_data(tester.deps_mut(), &nm, 0, another_data)
                    .is_ok());

                // original one works IF it's for next epoch
                assert!(storage
                    .submit_performance_data(tester.deps_mut(), &nm, 1, data)
                    .is_ok());

                let res = storage
                    .submit_performance_data(tester.deps_mut(), &nm, 0, data)
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::StalePerformanceSubmission {
                        epoch_id: 0,
                        node_id: 123,
                        last_epoch_id: 1,
                        last_node_id: 123,
                    }
                );

                Ok(())
            }

            #[test]
            fn its_not_possible_to_submit_data_out_of_order() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm = tester.addr_make("network-monitor");
                tester.authorise_network_monitor(&nm)?;

                let data = NodePerformance {
                    node_id: 123,
                    performance: Percent::hundred(),
                };
                let another_data = NodePerformance {
                    node_id: 124,
                    performance: Percent::hundred(),
                };

                assert!(storage
                    .submit_performance_data(tester.deps_mut(), &nm, 0, another_data)
                    .is_ok());

                let res = storage
                    .submit_performance_data(tester.deps_mut(), &nm, 0, data)
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::StalePerformanceSubmission {
                        epoch_id: 0,
                        node_id: 123,
                        last_epoch_id: 0,
                        last_node_id: 124,
                    }
                );

                // check across epochs
                assert!(storage
                    .submit_performance_data(tester.deps_mut(), &nm, 10, data)
                    .is_ok());

                let res = storage
                    .submit_performance_data(tester.deps_mut(), &nm, 9, data)
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::StalePerformanceSubmission {
                        epoch_id: 9,
                        node_id: 123,
                        last_epoch_id: 10,
                        last_node_id: 123,
                    }
                );
                Ok(())
            }

            #[test]
            fn its_not_possible_to_submit_data_for_past_epochs() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                tester.set_mixnet_epoch(10)?;

                let nm = tester.addr_make("network-monitor");
                tester.authorise_network_monitor(&nm)?;

                // if NM got authorised at epoch 10, it can only submit data for epochs >=10
                let res = storage
                    .submit_performance_data(tester.deps_mut(), &nm, 0, dummy_node_performance())
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::StalePerformanceSubmission {
                        epoch_id: 0,
                        node_id: 420,
                        last_epoch_id: 10,
                        last_node_id: 0,
                    }
                );

                let res = storage
                    .submit_performance_data(tester.deps_mut(), &nm, 9, dummy_node_performance())
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::StalePerformanceSubmission {
                        epoch_id: 9,
                        node_id: 420,
                        last_epoch_id: 10,
                        last_node_id: 0,
                    }
                );

                assert!(storage
                    .submit_performance_data(tester.deps_mut(), &nm, 10, dummy_node_performance())
                    .is_ok());
                assert!(storage
                    .submit_performance_data(tester.deps_mut(), &nm, 11, dummy_node_performance())
                    .is_ok());

                Ok(())
            }

            #[test]
            fn updates_submission_metadata() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let nm = tester.addr_make("network-monitor");
                tester.authorise_network_monitor(&nm)?;
                let metadata = storage
                    .performance_results
                    .submission_metadata
                    .load(&tester, &nm)?;
                assert_eq!(metadata.last_submitted_epoch_id, 0);
                assert_eq!(metadata.last_submitted_node_id, 0);

                storage.submit_performance_data(
                    tester.deps_mut(),
                    &nm,
                    0,
                    NodePerformance {
                        node_id: 1,
                        performance: Default::default(),
                    },
                )?;
                let metadata = storage
                    .performance_results
                    .submission_metadata
                    .load(&tester, &nm)?;
                assert_eq!(metadata.last_submitted_epoch_id, 0);
                assert_eq!(metadata.last_submitted_node_id, 1);

                storage.submit_performance_data(
                    tester.deps_mut(),
                    &nm,
                    0,
                    NodePerformance {
                        node_id: 42,
                        performance: Default::default(),
                    },
                )?;
                let metadata = storage
                    .performance_results
                    .submission_metadata
                    .load(&tester, &nm)?;
                assert_eq!(metadata.last_submitted_epoch_id, 0);
                assert_eq!(metadata.last_submitted_node_id, 42);

                storage.submit_performance_data(
                    tester.deps_mut(),
                    &nm,
                    1,
                    NodePerformance {
                        node_id: 3,
                        performance: Default::default(),
                    },
                )?;
                let metadata = storage
                    .performance_results
                    .submission_metadata
                    .load(&tester, &nm)?;
                assert_eq!(metadata.last_submitted_epoch_id, 1);
                assert_eq!(metadata.last_submitted_node_id, 3);

                storage.submit_performance_data(
                    tester.deps_mut(),
                    &nm,
                    12345,
                    NodePerformance {
                        node_id: 54321,
                        performance: Default::default(),
                    },
                )?;
                let metadata = storage
                    .performance_results
                    .submission_metadata
                    .load(&tester, &nm)?;
                assert_eq!(metadata.last_submitted_epoch_id, 12345);
                assert_eq!(metadata.last_submitted_node_id, 54321);

                Ok(())
            }
        }

        #[cfg(test)]
        mod batch_submitting_performance_data {
            use super::*;

            #[test]
            fn is_only_allowed_by_authorised_network_monitors() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm1 = tester.addr_make("network-monitor-1");
                let nm2 = tester.addr_make("network-monitor-2");
                let unauthorised = tester.addr_make("unauthorised");

                tester.authorise_network_monitor(&nm1)?;

                // authorised network monitor can submit the results just fine
                assert!(storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        &nm1,
                        0,
                        vec![dummy_node_performance()]
                    )
                    .is_ok());

                // unauthorised address is rejected
                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        &nm2,
                        0,
                        vec![dummy_node_performance()],
                    )
                    .unwrap_err();
                assert_eq!(
                    res,
                    NymPerformanceContractError::NotAuthorised {
                        address: nm2.clone()
                    }
                );

                // it is fine after explicit authorisation though
                tester.authorise_network_monitor(&nm2)?;
                assert!(storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        &nm2,
                        0,
                        vec![dummy_node_performance()]
                    )
                    .is_ok());

                // and address that was never authorised still fails
                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        &unauthorised,
                        0,
                        vec![dummy_node_performance()],
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
                let nm = tester.addr_make("network-monitor");
                tester.authorise_network_monitor(&nm)?;

                let data = NodePerformance {
                    node_id: 123,
                    performance: Percent::hundred(),
                };
                let another_data = NodePerformance {
                    node_id: 124,
                    performance: Percent::hundred(),
                };
                let more_data = NodePerformance {
                    node_id: 125,
                    performance: Percent::hundred(),
                };

                let duplicates = vec![data, data];
                let another_dups = vec![another_data, another_data];
                let unsorted = vec![another_data, data];
                let semi_sorted = vec![data, more_data, another_data];
                let sorted = vec![data, another_data, more_data];

                let res = storage
                    .batch_submit_performance_results(tester.deps_mut(), &nm, 0, duplicates)
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::UnsortedBatchSubmission);

                let res = storage
                    .batch_submit_performance_results(tester.deps_mut(), &nm, 0, another_dups)
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::UnsortedBatchSubmission);

                let res = storage
                    .batch_submit_performance_results(tester.deps_mut(), &nm, 0, unsorted)
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::UnsortedBatchSubmission);

                let res = storage
                    .batch_submit_performance_results(tester.deps_mut(), &nm, 0, semi_sorted)
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::UnsortedBatchSubmission);

                assert!(storage
                    .batch_submit_performance_results(tester.deps_mut(), &nm, 0, sorted)
                    .is_ok());
                Ok(())
            }

            #[test]
            fn its_not_possible_to_submit_data_for_same_node_again() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm = tester.addr_make("network-monitor");
                tester.authorise_network_monitor(&nm)?;

                let data = NodePerformance {
                    node_id: 123,
                    performance: Percent::hundred(),
                };
                let another_data = NodePerformance {
                    node_id: 124,
                    performance: Percent::hundred(),
                };

                // first submission
                assert!(storage
                    .batch_submit_performance_results(tester.deps_mut(), &nm, 0, vec![data])
                    .is_ok());

                // second submission
                let res = storage
                    .batch_submit_performance_results(tester.deps_mut(), &nm, 0, vec![data])
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::StalePerformanceSubmission {
                        epoch_id: 0,
                        node_id: 123,
                        last_epoch_id: 0,
                        last_node_id: 123,
                    }
                );

                // another submission works fine
                assert!(storage
                    .batch_submit_performance_results(tester.deps_mut(), &nm, 0, vec![another_data])
                    .is_ok());

                // original one works IF it's for next epoch
                assert!(storage
                    .batch_submit_performance_results(tester.deps_mut(), &nm, 1, vec![data])
                    .is_ok());

                let res = storage
                    .batch_submit_performance_results(tester.deps_mut(), &nm, 0, vec![data])
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::StalePerformanceSubmission {
                        epoch_id: 0,
                        node_id: 123,
                        last_epoch_id: 1,
                        last_node_id: 123,
                    }
                );

                Ok(())
            }

            #[test]
            fn its_not_possible_to_submit_data_out_of_order() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                let nm = tester.addr_make("network-monitor");
                tester.authorise_network_monitor(&nm)?;

                let data = NodePerformance {
                    node_id: 123,
                    performance: Percent::hundred(),
                };
                let another_data = NodePerformance {
                    node_id: 124,
                    performance: Percent::hundred(),
                };

                assert!(storage
                    .batch_submit_performance_results(tester.deps_mut(), &nm, 0, vec![another_data])
                    .is_ok());

                let res = storage
                    .batch_submit_performance_results(tester.deps_mut(), &nm, 0, vec![data])
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::StalePerformanceSubmission {
                        epoch_id: 0,
                        node_id: 123,
                        last_epoch_id: 0,
                        last_node_id: 124,
                    }
                );

                // check across epochs
                assert!(storage
                    .batch_submit_performance_results(tester.deps_mut(), &nm, 10, vec![data])
                    .is_ok());

                let res = storage
                    .batch_submit_performance_results(tester.deps_mut(), &nm, 9, vec![data])
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::StalePerformanceSubmission {
                        epoch_id: 9,
                        node_id: 123,
                        last_epoch_id: 10,
                        last_node_id: 123,
                    }
                );
                Ok(())
            }

            #[test]
            fn its_not_possible_to_submit_data_for_past_epochs() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();
                tester.set_mixnet_epoch(10)?;

                let nm = tester.addr_make("network-monitor");
                tester.authorise_network_monitor(&nm)?;

                // if NM got authorised at epoch 10, it can only submit data for epochs >=10
                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        &nm,
                        0,
                        vec![dummy_node_performance()],
                    )
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::StalePerformanceSubmission {
                        epoch_id: 0,
                        node_id: 420,
                        last_epoch_id: 10,
                        last_node_id: 0,
                    }
                );

                let res = storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        &nm,
                        9,
                        vec![dummy_node_performance()],
                    )
                    .unwrap_err();

                assert_eq!(
                    res,
                    NymPerformanceContractError::StalePerformanceSubmission {
                        epoch_id: 9,
                        node_id: 420,
                        last_epoch_id: 10,
                        last_node_id: 0,
                    }
                );

                assert!(storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        &nm,
                        10,
                        vec![dummy_node_performance()]
                    )
                    .is_ok());
                assert!(storage
                    .batch_submit_performance_results(
                        tester.deps_mut(),
                        &nm,
                        11,
                        vec![dummy_node_performance()]
                    )
                    .is_ok());

                Ok(())
            }

            #[test]
            fn updates_submission_metadata() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let nm = tester.addr_make("network-monitor");
                tester.authorise_network_monitor(&nm)?;
                let metadata = storage
                    .performance_results
                    .submission_metadata
                    .load(&tester, &nm)?;
                assert_eq!(metadata.last_submitted_epoch_id, 0);
                assert_eq!(metadata.last_submitted_node_id, 0);

                // single submission
                storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    &nm,
                    0,
                    vec![NodePerformance {
                        node_id: 1,
                        performance: Default::default(),
                    }],
                )?;
                let metadata = storage
                    .performance_results
                    .submission_metadata
                    .load(&tester, &nm)?;
                assert_eq!(metadata.last_submitted_epoch_id, 0);
                assert_eq!(metadata.last_submitted_node_id, 1);

                // another epoch
                storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    &nm,
                    1,
                    vec![NodePerformance {
                        node_id: 3,
                        performance: Default::default(),
                    }],
                )?;
                let metadata = storage
                    .performance_results
                    .submission_metadata
                    .load(&tester, &nm)?;
                assert_eq!(metadata.last_submitted_epoch_id, 1);
                assert_eq!(metadata.last_submitted_node_id, 3);

                // multiple submissions
                storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    &nm,
                    1,
                    vec![
                        NodePerformance {
                            node_id: 4,
                            performance: Default::default(),
                        },
                        NodePerformance {
                            node_id: 5,
                            performance: Default::default(),
                        },
                        NodePerformance {
                            node_id: 6,
                            performance: Default::default(),
                        },
                    ],
                )?;
                let metadata = storage
                    .performance_results
                    .submission_metadata
                    .load(&tester, &nm)?;
                assert_eq!(metadata.last_submitted_epoch_id, 1);
                assert_eq!(metadata.last_submitted_node_id, 6);

                // another epoch
                storage.batch_submit_performance_results(
                    tester.deps_mut(),
                    &nm,
                    2,
                    vec![
                        NodePerformance {
                            node_id: 2,
                            performance: Default::default(),
                        },
                        NodePerformance {
                            node_id: 75,
                            performance: Default::default(),
                        },
                        NodePerformance {
                            node_id: 123,
                            performance: Default::default(),
                        },
                    ],
                )?;
                let metadata = storage
                    .performance_results
                    .submission_metadata
                    .load(&tester, &nm)?;
                assert_eq!(metadata.last_submitted_epoch_id, 2);
                assert_eq!(metadata.last_submitted_node_id, 123);

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
            storage.initialise(deps, env, admin.clone(), mixnet_contract, Vec::new())?;

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
            storage.initialise(deps, env, admin.clone(), mixnet_contract, Vec::new())?;

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
                assert_eq!(
                    0,
                    storage
                        .performance_results
                        .submission_metadata
                        .load(&tester, &nm1)?
                        .last_submitted_epoch_id
                );

                tester.advance_mixnet_epoch()?;
                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm2.clone())?;
                assert_eq!(
                    1,
                    storage
                        .performance_results
                        .submission_metadata
                        .load(&tester, &nm2)?
                        .last_submitted_epoch_id
                );

                tester.set_mixnet_epoch(1000)?;
                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm3.clone())?;
                assert_eq!(
                    1000,
                    storage
                        .performance_results
                        .submission_metadata
                        .load(&tester, &nm3)?
                        .last_submitted_epoch_id
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

        #[test]
        fn loading_performance_data() -> anyhow::Result<()> {
            let storage = NymPerformanceContractStorage::new();
            let mut tester = init_contract_tester();
            let admin = tester.admin_unchecked();
            let mut nms = Vec::new();

            // pre-authorise some network monitors
            for i in 0..6 {
                let env = tester.env();
                let nm = tester.addr_make(&format!("network-monitor{i}"));
                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm.clone())?;
                nms.push(nm);
            }

            // no results
            assert_eq!(storage.try_load_performance(&tester, 0, 1)?, None);

            //
            // always returns median value with 2decimal places precision
            //

            // single result
            tester.insert_raw_performance(&nms[0], 1, "0.42")?;
            assert_eq!(
                storage
                    .try_load_performance(&tester, 0, 1)?
                    .unwrap()
                    .value()
                    .to_string(),
                "0.42"
            );

            // two results (median doesn't require changing decimal places)
            tester.insert_raw_performance(&nms[0], 2, "0.50")?;
            tester.insert_raw_performance(&nms[1], 2, "0.40")?;
            assert_eq!(
                storage
                    .try_load_performance(&tester, 0, 2)?
                    .unwrap()
                    .value()
                    .to_string(),
                "0.45"
            );

            // two results (median requires changing decimal places)
            tester.insert_raw_performance(&nms[0], 3, "0.58")?;
            tester.insert_raw_performance(&nms[1], 3, "0.45")?;
            assert_eq!(
                storage
                    .try_load_performance(&tester, 0, 3)?
                    .unwrap()
                    .value()
                    .to_string(),
                "0.52"
            );

            // three results (median is the middle value rather than the average)
            tester.insert_raw_performance(&nms[0], 4, "0.12")?;
            tester.insert_raw_performance(&nms[1], 4, "0.34")?;
            tester.insert_raw_performance(&nms[2], 4, "0.56")?;
            assert_eq!(
                storage
                    .try_load_performance(&tester, 0, 4)?
                    .unwrap()
                    .value()
                    .to_string(),
                "0.34"
            );

            // five results (notice how they're not inserted sorted)
            tester.insert_raw_performance(&nms[0], 5, "0.9")?;
            tester.insert_raw_performance(&nms[1], 5, "0.9")?;
            tester.insert_raw_performance(&nms[2], 5, "0.1")?;
            tester.insert_raw_performance(&nms[4], 5, "0.1")?;
            tester.insert_raw_performance(&nms[5], 5, "0.7")?;
            assert_eq!(
                storage
                    .try_load_performance(&tester, 0, 5)?
                    .unwrap()
                    .value()
                    .to_string(),
                "0.7"
            );

            // six results (same as above, but average of middle values)
            tester.insert_raw_performance(&nms[0], 6, "0.9")?;
            tester.insert_raw_performance(&nms[1], 6, "0.9")?;
            tester.insert_raw_performance(&nms[2], 6, "0.1")?;
            tester.insert_raw_performance(&nms[3], 6, "0.1")?;
            tester.insert_raw_performance(&nms[4], 6, "0.2")?;
            tester.insert_raw_performance(&nms[5], 6, "0.3")?;
            assert_eq!(
                storage
                    .try_load_performance(&tester, 0, 6)?
                    .unwrap()
                    .value()
                    .to_string(),
                "0.25"
            );

            Ok(())
        }

        #[cfg(test)]
        mod removing_node_measurements {
            use super::*;
            use cw_controllers::AdminError::NotAdmin;
            use nym_contracts_common_testing::FullReader;

            #[test]
            fn can_only_be_performed_by_contract_admin() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let admin = tester.admin_unchecked();
                let not_admin = tester.addr_make("not-admin");
                let nm = tester.addr_make("network-monitor");
                let env = tester.env();

                let epoch_id = 0;
                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm.clone())?;
                tester.insert_raw_performance(&nm, 1, "0.42")?;
                tester.insert_raw_performance(&nm, 2, "0.42")?;

                let res = storage
                    .remove_node_measurements(tester.deps_mut(), &not_admin, epoch_id, 1)
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::Admin(NotAdmin {}));

                assert!(storage
                    .remove_node_measurements(tester.deps_mut(), &admin, epoch_id, 1)
                    .is_ok());

                // change admin
                let new_admin = tester.addr_make("new-admin");
                tester.update_admin(&Some(new_admin.clone()))?;

                // old one no longer works
                let res = storage
                    .remove_node_measurements(tester.deps_mut(), &admin, epoch_id, 2)
                    .unwrap_err();
                assert_eq!(res, NymPerformanceContractError::Admin(NotAdmin {}));

                assert!(storage
                    .remove_node_measurements(tester.deps_mut(), &new_admin, epoch_id, 2)
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
                let nm1 = tester.addr_make("network-monitor1");
                let nm2 = tester.addr_make("network-monitor2");
                let nm3 = tester.addr_make("network-monitor3");

                let env = tester.env();

                let epoch_id = 0;
                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm1.clone())?;
                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm2.clone())?;
                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm3.clone())?;

                // single measurement
                tester.insert_raw_performance(&nm1, 1, "0.42")?;

                let before = storage
                    .performance_results
                    .results
                    .may_load(&tester, (epoch_id, 1))?;
                assert!(before.is_some());

                storage.remove_node_measurements(tester.deps_mut(), &admin, epoch_id, 1)?;

                let after = storage
                    .performance_results
                    .results
                    .may_load(&tester, (epoch_id, 1))?;
                assert!(after.is_none());

                // multiple measurements
                tester.insert_raw_performance(&nm1, 2, "0.42")?;
                tester.insert_raw_performance(&nm2, 2, "0.69")?;
                tester.insert_raw_performance(&nm3, 2, "1")?;

                let before = storage
                    .performance_results
                    .results
                    .may_load(&tester, (epoch_id, 2))?;
                assert!(before.is_some());

                storage.remove_node_measurements(tester.deps_mut(), &admin, epoch_id, 2)?;

                let after = storage
                    .performance_results
                    .results
                    .may_load(&tester, (epoch_id, 2))?;
                assert!(after.is_none());

                Ok(())
            }
        }

        #[cfg(test)]
        mod removing_epoch_measurements {
            use super::*;
            use cw_controllers::AdminError::NotAdmin;
            use nym_contracts_common_testing::FullReader;

            #[test]
            fn can_only_be_performed_by_contract_admin() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut tester = init_contract_tester();

                let admin = tester.admin_unchecked();
                let not_admin = tester.addr_make("not-admin");
                let nm = tester.addr_make("network-monitor");
                let env = tester.env();

                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm.clone())?;

                // epoch 0
                tester.insert_raw_performance(&nm, 1, "0.42")?;
                tester.insert_raw_performance(&nm, 2, "0.42")?;

                // epoch 1
                tester.advance_mixnet_epoch()?;
                tester.insert_raw_performance(&nm, 1, "0.42")?;
                tester.insert_raw_performance(&nm, 2, "0.42")?;

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
                let nm = tester.addr_make("network-monitor");

                let env = tester.env();
                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm.clone())?;

                // just few entries
                let epoch_id = 0;
                for i in 0..10 {
                    tester.insert_raw_performance(&nm, i + 1, "0.42")?;
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

                // EXACT limit
                let epoch_id = 1;
                tester.advance_mixnet_epoch()?;
                for i in 0..retrieval_limits::EPOCH_PERFORMANCE_PURGE_LIMIT {
                    tester.insert_raw_performance(&nm, (i + 1) as NodeId, "0.42")?;
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
                let nm = tester.addr_make("network-monitor");

                let env = tester.env();
                storage.authorise_network_monitor(tester.deps_mut(), &env, &admin, nm.clone())?;

                // just few entries
                let epoch_id = 0;
                for i in 0..2 * retrieval_limits::EPOCH_PERFORMANCE_PURGE_LIMIT + 50 {
                    tester.insert_raw_performance(&nm, (i + 1) as NodeId, "0.42")?;
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

            tester.authorise_network_monitor(&nm1)?;
            tester.authorise_network_monitor(&nm2)?;

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
        use crate::testing::{init_contract_tester, PerformanceContractTesterExt};
        use nym_contracts_common_testing::ContractOpts;
        use std::str::FromStr;

        #[test]
        fn inserting_new_entry() -> anyhow::Result<()> {
            // essentially make sure there are no silly bugs that epoch_id and node_id got accidentally mixed up
            // when constructing map key...
            let storage = PerformanceResultsStorage::new();
            let mut tester = init_contract_tester();

            let node_id1 = 123;
            let node_id2 = 456;

            let data1 = NodePerformance {
                node_id: node_id1,
                performance: Percent::from_str("0.23")?,
            };

            let data2 = NodePerformance {
                node_id: node_id1,
                performance: Percent::hundred(),
            };

            let data3 = NodePerformance {
                node_id: node_id2,
                performance: Percent::from_str("0.23643634")?,
            };

            let data4 = NodePerformance {
                node_id: node_id2,
                performance: Percent::hundred(),
            };

            assert!(storage.results.may_load(&tester, (1, node_id1))?.is_none());
            assert!(storage.results.may_load(&tester, (1, node_id2))?.is_none());

            storage.insert_performance_data(&mut tester, 1, &data1)?;
            assert_eq!(
                tester.read_raw_scores(1, node_id1)?.inner(),
                &[data1.performance]
            );
            storage.insert_performance_data(&mut tester, 1, &data2)?;
            assert_eq!(
                tester.read_raw_scores(1, node_id1)?.inner(),
                &[data1.performance, data2.performance]
            );

            storage.insert_performance_data(&mut tester, 1, &data3)?;
            assert_eq!(
                tester.read_raw_scores(1, node_id2)?.inner(),
                &[data3.performance.round_to_two_decimal_places()]
            );
            storage.insert_performance_data(&mut tester, 1, &data4)?;
            assert_eq!(
                tester.read_raw_scores(1, node_id2)?.inner(),
                &[
                    data3.performance.round_to_two_decimal_places(),
                    data4.performance
                ]
            );

            storage.insert_performance_data(&mut tester, 2, &data2)?;
            storage.insert_performance_data(&mut tester, 2, &data2)?;
            assert_eq!(
                tester.read_raw_scores(2, node_id1)?.inner(),
                &[data2.performance, data2.performance]
            );

            storage.insert_performance_data(&mut tester, 2, &data4)?;
            storage.insert_performance_data(&mut tester, 2, &data4)?;
            assert_eq!(
                tester.read_raw_scores(2, node_id2)?.inner(),
                &[data4.performance, data4.performance]
            );

            Ok(())
        }

        #[test]
        fn checking_for_submission_staleness() -> anyhow::Result<()> {
            let storage = PerformanceResultsStorage::new();
            let mut tester = init_contract_tester();

            let node_id = 123;

            let nm = tester.addr_make("network-monitor");
            tester.authorise_network_monitor(&nm)?;
            tester.insert_epoch_performance(&nm, 2, node_id, Percent::hundred())?;

            // illegal to submit anything < than last used epoch
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 0, node_id)
                .is_err());
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 1, node_id)
                .is_err());
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 1, node_id + 1)
                .is_err());

            // for the current epoch, node id has to be greater than what has already been submitted
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 2, 0)
                .is_err());
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 2, 122)
                .is_err());
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 2, node_id)
                .is_err());
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 2, node_id + 1)
                .is_ok());

            // and anything for future epochs is fine (as long as it's the first entry)
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 3, 0)
                .is_ok());
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 3, node_id)
                .is_ok());
            assert!(storage
                .ensure_non_stale_submission(&tester, &nm, 1111, 5454)
                .is_ok());

            Ok(())
        }
    }
}
