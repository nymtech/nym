// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::MixnetContractQuerier;
use cosmwasm_std::{Addr, Deps, DepsMut, Env, StdError, Storage};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use nym_performance_contract_common::constants::storage_keys;
use nym_performance_contract_common::{
    EpochId, NetworkMonitorDetails, NetworkMonitorSubmissionMetadata, NodeId, NodePerformance,
    NodeResults, NymPerformanceContractError, RetiredNetworkMonitor,
};

pub const NYM_PERFORMANCE_CONTRACT_STORAGE: NymPerformanceContractStorage =
    NymPerformanceContractStorage::new();

pub struct NymPerformanceContractStorage {
    pub(crate) contract_admin: Admin,

    pub(crate) mixnet_contract_address: Item<Addr>,

    pub(crate) network_monitors: NetworkMonitorsStorage,

    pub(crate) performance_results: PerformanceResultsStorage,
}

impl NymPerformanceContractStorage {
    #[allow(clippy::new_without_default)]
    const fn new() -> Self {
        NymPerformanceContractStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
            mixnet_contract_address: Item::new(storage_keys::MIXNET_CONTRACT),
            network_monitors: NetworkMonitorsStorage::new(),
            performance_results: PerformanceResultsStorage::new(),
        }
    }

    pub fn initialise(
        &self,
        mut deps: DepsMut,
        env: Env,
        admin: Addr,
        mixnet_contract_address: Addr,
        initial_authorised_network_monitors: Vec<String>,
    ) -> Result<(), NymPerformanceContractError> {
        let _ = deps;
        let _ = env;

        // set the contract admin
        self.contract_admin.set(deps.branch(), Some(admin))?;

        // set the mixnet contract address
        self.mixnet_contract_address
            .save(deps.storage, &mixnet_contract_address)?;

        // initialise the network monitors storage (by setting the current count to 0)
        self.network_monitors.initialise(deps.branch())?;

        // add all initial network monitors

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

        // not point in using peekable iterator, we can just keep track of the previous
        // element we've seen
        let mut previous = first.node_id;

        for perf in &data {
            // 3. ensure provided data is sorted (if the check fails in later iteration,
            // the whole tx will get reverted so it's fine to just set the storage within the same loop
            if perf.node_id <= previous {
                return Err(NymPerformanceContractError::UnsortedBatchSubmission);
            }
            previous = perf.node_id;

            // 4. insert performance data into the storage
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
        env: Env,
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
        let mixnet_contract_address = self.mixnet_contract_address.load(deps.storage)?;
        let current_epoch_id = deps
            .querier
            .query_current_absolute_mixnet_epoch_id(&mixnet_contract_address)?;

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
        env: Env,
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
        let performance = data.performance.round_to_two_decimal_places();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod performance_contract_storage {
        use super::*;
        use cosmwasm_std::testing::{mock_dependencies, mock_env};

        #[cfg(test)]
        mod initialisation {
            use super::*;
            use cosmwasm_std::testing::{mock_dependencies, mock_env};

            #[test]
            fn sets_contract_admin() -> anyhow::Result<()> {
                let storage = NymPerformanceContractStorage::new();
                let mut deps = mock_dependencies();
                let env = mock_env();
                let admin1 = deps.api.addr_make("first-admin");
                let admin2 = deps.api.addr_make("second-admin");
                let mixnet_contract = deps.api.addr_make("mixnet-contract");

                storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin1.clone(),
                    mixnet_contract.clone(),
                    Vec::new(),
                )?;
                assert!(storage.ensure_is_admin(deps.as_ref(), &admin1).is_ok());

                let mut deps = mock_dependencies();
                storage.initialise(
                    deps.as_mut(),
                    env.clone(),
                    admin2.clone(),
                    mixnet_contract,
                    Vec::new(),
                )?;
                assert!(storage.ensure_is_admin(deps.as_ref(), &admin2).is_ok());

                Ok(())
            }
        }

        #[test]
        fn checking_for_admin() -> anyhow::Result<()> {
            let storage = NymPerformanceContractStorage::new();
            let mut deps = mock_dependencies();
            let env = mock_env();
            let admin = deps.api.addr_make("admin");
            let non_admin = deps.api.addr_make("non-admin");
            let mixnet_contract = deps.api.addr_make("mixnet-contract");

            storage.initialise(
                deps.as_mut(),
                env,
                admin.clone(),
                mixnet_contract,
                Vec::new(),
            )?;
            assert!(storage.is_admin(deps.as_ref(), &admin)?);
            assert!(!storage.is_admin(deps.as_ref(), &non_admin)?);

            Ok(())
        }

        #[test]
        fn ensuring_admin_privileges() -> anyhow::Result<()> {
            let storage = NymPerformanceContractStorage::new();
            let mut deps = mock_dependencies();
            let env = mock_env();
            let admin = deps.api.addr_make("admin");
            let non_admin = deps.api.addr_make("non-admin");
            let mixnet_contract = deps.api.addr_make("mixnet-contract");

            storage.initialise(
                deps.as_mut(),
                env,
                admin.clone(),
                mixnet_contract,
                Vec::new(),
            )?;
            assert!(storage.ensure_is_admin(deps.as_ref(), &admin).is_ok());
            assert!(storage.ensure_is_admin(deps.as_ref(), &non_admin).is_err());

            Ok(())
        }
    }
}
