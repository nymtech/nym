// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::contract::{execute, instantiate, migrate, query};
use node_families_contract_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NodeFamiliesContractError, QueryMsg,
};
use nym_contracts_common_testing::{
    AdminExt, ChainOpts, ContractFn, ContractOpts, ContractTester, DenomExt, PermissionedFn,
    QueryFn, RandExt, TestableNymContract,
};

pub struct NodeFamiliesContract;

impl TestableNymContract for NodeFamiliesContract {
    const NAME: &'static str = "node-families-contract";
    type InitMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;
    type MigrateMsg = MigrateMsg;
    type ContractError = NodeFamiliesContractError;

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
}

pub fn init_contract_tester() -> ContractTester<NodeFamiliesContract> {
    NodeFamiliesContract::init()
}

pub trait NodeFamiliesContractTesterExt:
    ContractOpts<
        ExecuteMsg = ExecuteMsg,
        QueryMsg = QueryMsg,
        ContractError = NodeFamiliesContractError,
    > + ChainOpts
    + AdminExt
    + DenomExt
    + RandExt
{
    //
}

impl NodeFamiliesContractTesterExt for ContractTester<NodeFamiliesContract> {}
