// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::{execute, instantiate, migrate, query};
use crate::error::ContractError;
use nym_contracts_common_testing::{ContractFn, PermissionedFn, QueryFn};

pub use nym_contracts_common_testing::TestableNymContract;
pub use nym_group_contract_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

pub struct GroupContract;

impl TestableNymContract for GroupContract {
    const NAME: &'static str = "cw4-group-contract";
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
        |deps, env, msg| query(deps, env, msg).map_err(Into::into)
    }

    fn migrate() -> PermissionedFn<Self::MigrateMsg, Self::ContractError> {
        migrate
    }

    fn base_init_msg() -> Self::InitMsg {
        unimplemented!()
    }
}
