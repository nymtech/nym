// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::{execute, instantiate, migrate, query};
use cosmwasm_std::{Addr, Order};
use nym_contracts_common_testing::{
    mock_dependencies, AdminExt, ChainOpts, CommonStorageKeys, ContractFn, ContractOpts,
    ContractTester, DenomExt, PermissionedFn, QueryFn, RandExt, Rng, TestableNymContract,
};
use std::net::{IpAddr, Ipv4Addr};

use crate::storage::NetworkMonitorsStorage;
use nym_network_monitors_contract_common::constants::storage_keys;
use nym_network_monitors_contract_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NetworkMonitorsContractError, QueryMsg,
};

pub struct NetworkMonitorsContract;

impl TestableNymContract for NetworkMonitorsContract {
    const NAME: &'static str = "nym-network-monitors-contract";
    type InitMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;
    type MigrateMsg = MigrateMsg;
    type ContractError = NetworkMonitorsContractError;

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
        let deps = mock_dependencies();
        InstantiateMsg {
            orchestrator_address: deps.api.addr_make("initial-dummy-orchestrator").to_string(),
        }
    }
}

pub fn init_contract_tester() -> ContractTester<NetworkMonitorsContract> {
    NetworkMonitorsContract::init()
        .with_common_storage_key(CommonStorageKeys::Admin, storage_keys::CONTRACT_ADMIN)
}

pub trait NetworkMonitorsContractTesterExt:
    ContractOpts<
        ExecuteMsg = ExecuteMsg,
        QueryMsg = QueryMsg,
        ContractError = NetworkMonitorsContractError,
    > + ChainOpts
    + AdminExt
    + DenomExt
    + RandExt
{
    fn add_orchestrator(&mut self) -> Result<Addr, NetworkMonitorsContractError> {
        let admin = self.admin_unchecked();
        let addr = self.generate_account();
        self.execute_raw(
            admin,
            ExecuteMsg::AuthoriseNetworkMonitorOrchestrator {
                address: addr.to_string(),
            },
        )?;
        Ok(addr)
    }

    fn remove_all_orchestrators(&mut self) {
        let orchestrators = self.all_orchestrators();
        for orchestrator in orchestrators {
            self.execute_raw(
                self.admin_unchecked(),
                ExecuteMsg::RevokeNetworkMonitorOrchestrator {
                    address: orchestrator.to_string(),
                },
            )
            .unwrap();
        }
    }

    fn random_ip(&mut self) -> IpAddr {
        let rng = self.raw_rng();
        IpAddr::V4(Ipv4Addr::new(rng.gen(), rng.gen(), rng.gen(), rng.gen()))
    }

    fn all_agents(&self) -> Vec<IpAddr> {
        NetworkMonitorsStorage::new()
            .authorised_agents
            .range(self.storage(), None, None, Order::Ascending)
            .map(|record| record.unwrap().0.parse().unwrap())
            .collect()
    }

    fn all_orchestrators(&self) -> Vec<Addr> {
        NetworkMonitorsStorage::new()
            .authorised_orchestrators
            .range(self.storage(), None, None, Order::Ascending)
            .map(|record| record.unwrap().0)
            .collect()
    }
}

impl NetworkMonitorsContractTesterExt for ContractTester<NetworkMonitorsContract> {}
