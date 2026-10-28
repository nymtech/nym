// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::{execute, instantiate, migrate, query};
use crate::storage::NYM_PERFORMANCE_CONTRACT_STORAGE;
use cosmwasm_std::testing::{mock_env, MockApi};
use cosmwasm_std::{Addr, ContractInfo, Deps, DepsMut, Env, QuerierWrapper};
use mixnet_contract::testable_mixnet_contract::{
    EmbeddedMixnetContractExt, MixnetContract, MixnetContractSiblings,
};
use nym_contracts_common::Percent;
use nym_contracts_common_testing::{
    addr, AdminExt, ArbitraryContractStorageReader, ArbitraryContractStorageWriter, BankExt,
    ChainOpts, CommonStorageKeys, ContractFn, ContractOpts, ContractStorageWrapper, ContractTester,
    ContractTesterBuilder, DenomExt, PermissionedFn, QueryFn, RandExt, TestableNymContract,
};
use nym_mixnet_contract_common::{EpochId, EpochState, EpochStatus};
use nym_performance_contract_common::constants::storage_keys;
use nym_performance_contract_common::{
    EpochNodeMeasurements, ExecuteMsg, InstantiateMsg, Measurements, MigrateMsg,
    NetworkMonitorSubmissionMetadata, NodeId, NodeResults, NodeSubmission,
    NymPerformanceContractError, QueryMsg, Weights,
};

pub struct PerformanceContract;

impl TestableNymContract for PerformanceContract {
    const NAME: &'static str = "performance-contract";
    type InitMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;
    type MigrateMsg = MigrateMsg;
    type ContractError = NymPerformanceContractError;

    fn instantiate() -> ContractFn<Self::InitMsg, Self::ContractError> {
        instantiate
    }

    fn execute() -> ContractFn<Self::ExecuteMsg, Self::ContractError> {
        execute
    }

    fn query() -> QueryFn<Self::QueryMsg, Self::ContractError> {
        query
    }

    fn migrate() -> PermissionedFn<Self::MigrateMsg, Self::ContractError> {
        migrate
    }

    fn base_init_msg() -> Self::InitMsg {
        InstantiateMsg {
            mixnet_contract_address: addr("mixnet-contract").to_string(),
            authorised_network_monitors: vec![],
            initial_weights: liveness_only_weights(),
        }
    }

    fn init() -> ContractTester<Self>
    where
        Self: Sized,
    {
        let builder = ContractTesterBuilder::new().instantiate::<MixnetContract>(None);

        // we just instantiated it
        let mixnet_address = builder
            .well_known_contracts
            .get(MixnetContract::NAME)
            .unwrap()
            .clone();

        builder
            .instantiate::<Self>(Some(InstantiateMsg {
                mixnet_contract_address: mixnet_address.to_string(),
                authorised_network_monitors: vec![],
                initial_weights: liveness_only_weights(),
            }))
            .build()
    }
}

pub fn init_contract_tester() -> ContractTester<PerformanceContract> {
    let mut tester = PerformanceContract::init()
        .with_common_storage_key(CommonStorageKeys::Admin, storage_keys::CONTRACT_ADMIN);

    // remove placeholder addresses for node families and geolocation contracts from the mixnet
    // contract so that their on unbond hooks don't get invoked
    tester
        .set_mixnet_sibling_contracts(MixnetContractSiblings::default().with_clear_all())
        .expect("should be able to patch mixnet contract state");

    tester
}

/// The weights every test instantiates with unless it says otherwise.
pub(crate) fn liveness_only_weights() -> Weights {
    Weights {
        liveness: Percent::hundred(),
        stress: Percent::zero(),
    }
}

/// Parses a percent literal such as `"0.42"`.
pub(crate) fn p(raw: &str) -> Percent {
    raw.parse().expect("invalid percent literal")
}

/// The stored values of one kind, ascending.
pub(crate) fn values(results: &NodeResults) -> Vec<Percent> {
    results.values().collect()
}

/// A liveness-only submission for `node_id`.
pub(crate) fn liveness_submission(node_id: NodeId, raw: &str) -> NodeSubmission {
    NodeSubmission {
        node_id,
        measurements: Measurements::default().with_liveness(p(raw)),
    }
}

/// A submission carrying liveness and config, i.e. one that scores.
pub(crate) fn scored_submission(node_id: NodeId, liveness: &str, config: &str) -> NodeSubmission {
    NodeSubmission {
        node_id,
        measurements: Measurements::default()
            .with_liveness(p(liveness))
            .with_config(p(config)),
    }
}

// we need to be able to test instantiation, but for that we require
// deps in a state that already includes instantiated mixnet contract
pub(crate) struct PreInitContract {
    tester_builder: ContractTesterBuilder<PerformanceContract>,
    pub(crate) mixnet_contract_address: Addr,
    pub(crate) api: MockApi,
    storage: ContractStorageWrapper,
    placeholder_address: Addr,
}

impl PreInitContract {
    pub(crate) fn new() -> PreInitContract {
        let tester_builder =
            ContractTesterBuilder::<PerformanceContract>::new().instantiate::<MixnetContract>(None);

        let mixnet_contract = tester_builder
            .well_known_contracts
            .get(&MixnetContract::NAME)
            .unwrap();

        let api = tester_builder.api();
        let placeholder_address = api.addr_make("to-be-performance-contract");

        let storage = tester_builder.contract_storage_wrapper(&placeholder_address);

        PreInitContract {
            mixnet_contract_address: mixnet_contract.clone(),
            tester_builder,
            api,
            storage,
            placeholder_address,
        }
    }

    pub(crate) fn deps(&self) -> Deps<'_> {
        Deps {
            storage: &self.storage,
            api: &self.api,
            querier: self.tester_builder.querier(),
        }
    }

    pub(crate) fn deps_mut(&mut self) -> DepsMut<'_> {
        DepsMut {
            storage: &mut self.storage,
            api: &self.api,
            querier: self.tester_builder.querier(),
        }
    }

    pub(crate) fn querier(&self) -> QuerierWrapper<'_> {
        self.tester_builder.querier()
    }

    pub(crate) fn env(&self) -> Env {
        Env {
            contract: ContractInfo {
                address: self.placeholder_address.clone(),
            },
            ..mock_env()
        }
    }

    pub(crate) fn addr_make(&self, input: &str) -> Addr {
        self.api.addr_make(input)
    }
}

impl ArbitraryContractStorageWriter for PreInitContract {
    fn set_contract_storage(
        &mut self,
        address: impl Into<String>,
        key: impl AsRef<[u8]>,
        value: impl AsRef<[u8]>,
    ) {
        self.storage
            .as_inner_storage_mut()
            .set_contract_storage(address, key, value);
    }
}

pub(crate) trait PerformanceContractTesterExt:
    ContractOpts<
        ExecuteMsg = ExecuteMsg,
        QueryMsg = QueryMsg,
        ContractError = NymPerformanceContractError,
    > + ChainOpts
    + AdminExt
    + DenomExt
    + RandExt
    + BankExt
    + ArbitraryContractStorageReader
    + ArbitraryContractStorageWriter
    + EmbeddedMixnetContractExt
{
    fn authorise_network_monitor(&mut self, addr: &Addr) {
        let admin = self.admin_unchecked();
        self.execute_raw(
            admin,
            ExecuteMsg::AuthoriseNetworkMonitor {
                address: addr.to_string(),
            },
        )
        .unwrap();
    }

    /// Generates a fresh account and authorises it as a network monitor.
    fn new_authorised_network_monitor(&mut self) -> Addr {
        let network_monitor = self.generate_account();
        self.authorise_network_monitor(&network_monitor);
        network_monitor
    }

    /// Bonds `count` dummy nodes and returns their ids in bonding order.
    fn bond_dummy_nymnodes(&mut self, count: usize) -> Vec<NodeId> {
        (0..count)
            .map(|_| self.bond_dummy_nymnode().unwrap())
            .collect()
    }

    /// Bonds a fresh node and returns a liveness-only submission for it.
    fn dummy_node_submission(&mut self) -> NodeSubmission {
        let node_id = self.bond_dummy_nymnode().unwrap();
        liveness_submission(node_id, "0.69")
    }

    /// Submits through the contract entry point as `addr` for an explicit epoch.
    fn submit_for_epoch(&mut self, addr: &Addr, epoch: EpochId, data: NodeSubmission) {
        self.execute_raw(addr.clone(), ExecuteMsg::Submit { epoch, data })
            .unwrap();
    }

    /// Submits for the current mixnet epoch.
    fn submit_now(&mut self, addr: &Addr, data: NodeSubmission) {
        let epoch = self.current_mixnet_epoch().unwrap();
        self.submit_for_epoch(addr, epoch, data);
    }

    /// Moves the mixnet to `epoch` and submits for it.
    fn submit_at_epoch(&mut self, addr: &Addr, epoch: EpochId, data: NodeSubmission) {
        self.set_mixnet_epoch(epoch).unwrap();
        self.submit_for_epoch(addr, epoch, data);
    }

    /// Submits a liveness-only value for the current mixnet epoch.
    fn submit_liveness(&mut self, addr: &Addr, node_id: NodeId, raw: &str) {
        self.submit_now(addr, liveness_submission(node_id, raw));
    }

    /// Submits liveness with a config of 100% for the current mixnet epoch, so it scores.
    fn submit_scored(&mut self, addr: &Addr, node_id: NodeId, liveness: &str) {
        self.submit_now(addr, scored_submission(node_id, liveness, "1"));
    }

    /// The stored bundle for the node in the epoch.
    fn read_bundle(&self, epoch_id: EpochId, node_id: NodeId) -> EpochNodeMeasurements {
        NYM_PERFORMANCE_CONTRACT_STORAGE
            .performance_results
            .results
            .load(self.deps().storage, (epoch_id, node_id))
            .unwrap()
    }

    /// The monitor's replay cursor: the last epoch and node it submitted.
    fn submission_metadata(&self, addr: &Addr) -> NetworkMonitorSubmissionMetadata {
        NYM_PERFORMANCE_CONTRACT_STORAGE
            .performance_results
            .submission_metadata
            .load(self.deps().storage, addr)
            .unwrap()
    }

    /// The latest epoch the node has a bundle for, if any.
    fn last_known_epoch(&self, node_id: NodeId) -> Option<EpochId> {
        NYM_PERFORMANCE_CONTRACT_STORAGE
            .performance_results
            .last_known_epoch
            .may_load(self.deps().storage, node_id)
            .unwrap()
    }

    /// Patches the mixnet contract's epoch status, e.g. to seal the current epoch.
    fn set_mixnet_epoch_status(&mut self, state: EpochState) {
        let being_advanced_by = self.addr_make("rewarder");
        self.write_to_mixnet_contract_storage_value(
            b"ces",
            &EpochStatus {
                being_advanced_by,
                state,
            },
        )
        .unwrap();
    }
}

impl PerformanceContractTesterExt for ContractTester<PerformanceContract> {}
