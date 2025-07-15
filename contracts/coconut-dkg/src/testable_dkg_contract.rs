// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// fine in test code
#![allow(clippy::unwrap_used)]

use crate::contract::{execute, instantiate, migrate, query};
use crate::error::ContractError;
use nym_contracts_common_testing::{ContractFn, ContractTester, PermissionedFn, QueryFn};

pub use nym_coconut_dkg_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
pub use nym_contracts_common_testing::TestableNymContract;

pub struct DkgContract;

impl TestableNymContract for DkgContract {
    const NAME: &'static str = "dkg-contract";
    type InitMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;
    type MigrateMsg = MigrateMsg;
    type ContractError = ContractError;

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
        unimplemented!()
    }

    // NOTE: for proper integration tests the below would have to be implemented
    // similarly to the offline signers contract with proper dependencies on cw3 and cw4 contracts
    fn init() -> ContractTester<Self>
    where
        Self: Sized,
    {
        unimplemented!()
    }
}
