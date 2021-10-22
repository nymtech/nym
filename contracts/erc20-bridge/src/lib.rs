// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod error;
mod queries;
mod storage;
mod transactions;

use cosmwasm_std::{
    entry_point, to_binary, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response,
};

use crate::error::ContractError;
use erc20_bridge_contract::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

/// Instantiate the contract.
///
/// `deps` contains Storage, API and Querier
/// `env` contains block, message and contract info
/// `msg` is the contract initialization message, sort of like a constructor call.
#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

/// Handle an incoming message
#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::LinkPayment { data } => transactions::link_payment(deps, env, info, data),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    let query_res = match msg {
        QueryMsg::GetPayments { start_after, limit } => {
            to_binary(&queries::query_payments_paged(deps, start_after, limit)?)
        }
    };

    Ok(query_res?)
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Default::default())
}
