// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract;
use cosmwasm_contract_testing::TestableContract;
use cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, QueryResponse, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{ExecuteMsg, InstantiateMsg, QueryMsg};

pub struct MixnetContract;

impl TestableContract for MixnetContract {
    type ContractError = MixnetContractError;
    type InstantiateMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;

    fn new() -> Self {
        MixnetContract
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
