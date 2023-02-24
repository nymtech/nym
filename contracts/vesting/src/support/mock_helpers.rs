// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract;
use crate::errors::ContractError;
use cosmwasm_contract_testing::TestableContract;
use cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, QueryResponse, Response};
use vesting_contract_common::{ExecuteMsg, InitMsg, QueryMsg};

pub struct VestingContract;

impl TestableContract for VestingContract {
    type ContractError = ContractError;
    type InstantiateMsg = InitMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;

    fn new() -> Self {
        VestingContract
    }

    fn instantiate(
        deps: DepsMut<'_>,
        env: Env,
        info: MessageInfo,
        msg: Self::InstantiateMsg,
    ) -> Result<Response, Self::ContractError> {
        contract::instantiate(deps, env, info, msg)
    }

    fn execute(
        deps: DepsMut<'_>,
        env: Env,
        info: MessageInfo,
        msg: Self::ExecuteMsg,
    ) -> Result<Response, Self::ContractError> {
        contract::execute(deps, env, info, msg)
    }

    fn query(
        deps: Deps<'_>,
        env: Env,
        msg: Self::QueryMsg,
    ) -> Result<QueryResponse, Self::ContractError> {
        contract::query(deps, env, msg)
    }
}
