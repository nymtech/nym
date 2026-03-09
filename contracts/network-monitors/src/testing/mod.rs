// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::{execute, instantiate, migrate, query};
use nym_contracts_common_testing::{
    AdminExt, ChainOpts, CommonStorageKeys, ContractFn, ContractOpts, ContractTester, DenomExt,
    PermissionedFn, QueryFn, RandExt, TestableNymContract,
};

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
        InstantiateMsg {}
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
    //
}

impl NetworkMonitorsContractTesterExt for ContractTester<NetworkMonitorsContract> {}
