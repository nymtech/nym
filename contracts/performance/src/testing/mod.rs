// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::{execute, instantiate, migrate, query};
use crate::helpers::MixnetContractQuerier;
use crate::storage::NYM_PERFORMANCE_CONTRACT_STORAGE;
use cosmwasm_std::testing::{mock_env, MockApi};
use cosmwasm_std::{Addr, ContractInfo, Deps, DepsMut, Env, QuerierWrapper, StdError, StdResult};
use mixnet_contract::testable_mixnet_contract::MixnetContract;
use nym_contracts_common::Percent;
use nym_contracts_common_testing::{
    addr, AdminExt, ArbitraryContractStorageWriter, ChainOpts, CommonStorageKeys, ContractFn,
    ContractOpts, ContractStorageWrapper, ContractTester, ContractTesterBuilder, DenomExt,
    PermissionedFn, QueryFn, RandExt, TestableNymContract,
};
use nym_mixnet_contract_common::{EpochId, Interval};
use nym_performance_contract_common::constants::storage_keys;
use nym_performance_contract_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NodeId, NodePerformance, NodeResults,
    NymPerformanceContractError, QueryMsg,
};
use serde::Serialize;
use std::str::FromStr;

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
            }))
            .build()
    }
}

pub fn init_contract_tester() -> ContractTester<PerformanceContract> {
    PerformanceContract::init()
        .with_common_storage_key(CommonStorageKeys::Admin, storage_keys::CONTRACT_ADMIN)
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

#[allow(dead_code)]
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

    pub(crate) fn deps(&self) -> Deps {
        Deps {
            storage: &self.storage,
            api: &self.api,
            querier: self.tester_builder.querier(),
        }
    }

    pub(crate) fn deps_mut(&mut self) -> DepsMut {
        DepsMut {
            storage: &mut self.storage,
            api: &self.api,
            querier: self.tester_builder.querier(),
        }
    }

    pub(crate) fn querier(&self) -> QuerierWrapper {
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

    pub(crate) fn write_to_mixnet_contract_storage(
        &mut self,
        key: impl AsRef<[u8]>,
        value: impl AsRef<[u8]>,
    ) -> StdResult<()> {
        let address = NYM_PERFORMANCE_CONTRACT_STORAGE
            .mixnet_contract_address
            .load(self.deps().storage)?;

        self.set_contract_storage(address, key, value);
        Ok(())
    }

    pub(crate) fn write_to_mixnet_contract_storage_value<T: Serialize>(
        &mut self,
        key: impl AsRef<[u8]>,
        value: &T,
    ) -> StdResult<()> {
        let address = NYM_PERFORMANCE_CONTRACT_STORAGE
            .mixnet_contract_address
            .load(self.deps().storage)?;

        self.set_contract_storage_value(address, key, value)
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

#[allow(dead_code)]
pub(crate) trait PerformanceContractTesterExt:
    ContractOpts<
        ExecuteMsg = ExecuteMsg,
        QueryMsg = QueryMsg,
        ContractError = NymPerformanceContractError,
    > + ChainOpts
    + AdminExt
    + DenomExt
    + RandExt
    + ArbitraryContractStorageWriter
{
    fn write_to_mixnet_contract_storage(
        &mut self,
        key: impl AsRef<[u8]>,
        value: impl AsRef<[u8]>,
    ) -> StdResult<()> {
        let address = NYM_PERFORMANCE_CONTRACT_STORAGE
            .mixnet_contract_address
            .load(self.deps().storage)?;

        <Self as ArbitraryContractStorageWriter>::set_contract_storage(self, address, key, value);
        Ok(())
    }

    fn write_to_mixnet_contract_storage_value<T: Serialize>(
        &mut self,
        key: impl AsRef<[u8]>,
        value: &T,
    ) -> StdResult<()> {
        let address = NYM_PERFORMANCE_CONTRACT_STORAGE
            .mixnet_contract_address
            .load(self.deps().storage)?;

        self.set_contract_storage_value(address, key, value)
    }

    fn advance_mixnet_epoch(&mut self) -> StdResult<()> {
        let address = NYM_PERFORMANCE_CONTRACT_STORAGE
            .mixnet_contract_address
            .load(self.deps().storage)?;

        let current = self
            .deps()
            .querier
            .query_current_mixnet_interval(address.clone())?;
        self.set_contract_storage_value(&address, b"ci", &current.advance_epoch())
    }

    fn set_mixnet_epoch(&mut self, epoch_id: EpochId) -> StdResult<()> {
        let address = NYM_PERFORMANCE_CONTRACT_STORAGE
            .mixnet_contract_address
            .load(self.deps().storage)?;

        let interval = self
            .deps()
            .querier
            .query_current_mixnet_interval(address.clone())?;

        let mut to_update = if interval.current_epoch_absolute_id() <= epoch_id {
            interval
        } else {
            Interval::init_interval(
                interval.epochs_in_interval(),
                interval.epoch_length(),
                &mock_env(),
            )
        };

        let current = to_update.current_epoch_absolute_id();
        let diff = epoch_id - current;
        for _ in 0..diff {
            to_update = to_update.advance_epoch();
        }
        self.set_contract_storage_value(&address, b"ci", &to_update)
    }

    fn authorise_network_monitor(
        &mut self,
        addr: &Addr,
    ) -> Result<(), NymPerformanceContractError> {
        let admin = self.admin_unchecked();
        self.execute_raw(
            admin,
            ExecuteMsg::AuthoriseNetworkMonitor {
                address: addr.to_string(),
            },
        )?;
        Ok(())
    }

    fn retire_network_monitor(&mut self, addr: &Addr) -> Result<(), NymPerformanceContractError> {
        let admin = self.admin_unchecked();
        self.execute_raw(
            admin,
            ExecuteMsg::RetireNetworkMonitor {
                address: addr.to_string(),
            },
        )?;
        Ok(())
    }

    fn insert_epoch_performance(
        &mut self,
        addr: &Addr,
        epoch_id: EpochId,
        node_id: NodeId,
        performance: Percent,
    ) -> Result<(), NymPerformanceContractError> {
        NYM_PERFORMANCE_CONTRACT_STORAGE.submit_performance_data(
            self.deps_mut(),
            addr,
            epoch_id,
            NodePerformance {
                node_id,
                performance,
            },
        )
    }

    fn insert_performance(
        &mut self,
        addr: &Addr,
        node_id: NodeId,
        performance: Percent,
    ) -> Result<(), NymPerformanceContractError> {
        let address = NYM_PERFORMANCE_CONTRACT_STORAGE
            .mixnet_contract_address
            .load(self.deps().storage)?;

        let epoch_id = self
            .deps()
            .querier
            .query_current_mixnet_interval(address.clone())?
            .current_epoch_absolute_id();

        self.insert_epoch_performance(addr, epoch_id, node_id, performance)
    }

    // makes testing easier
    fn insert_raw_performance(
        &mut self,
        addr: &Addr,
        node_id: NodeId,
        raw: &str,
    ) -> Result<(), NymPerformanceContractError> {
        self.insert_performance(
            addr,
            node_id,
            Percent::from_str(raw).map_err(|err| {
                NymPerformanceContractError::StdErr(StdError::parse_err("Percent", err.to_string()))
            })?,
        )
    }

    fn read_raw_scores(
        &self,
        epoch_id: EpochId,
        node_id: NodeId,
    ) -> Result<NodeResults, NymPerformanceContractError> {
        let scores = NYM_PERFORMANCE_CONTRACT_STORAGE
            .performance_results
            .results
            .load(self.deps().storage, (epoch_id, node_id))?;
        Ok(scores)
    }
}

impl PerformanceContractTesterExt for ContractTester<PerformanceContract> {}
