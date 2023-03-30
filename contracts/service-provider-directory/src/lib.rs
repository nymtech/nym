//! The service provider directory contract is for users to announce their service providers for
//! public use.

use crate::error::Result;
use nym_service_provider_directory_common::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response};
use error::ContractError;

mod contract;
mod error;
mod state;

pub mod constants;

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod test_helpers;

/// Contract entry point for instantiation.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response> {
    contract::instantiate(deps, env, info, msg)
}

/// Contract entry point for execution
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    contract::execute(deps, env, info, msg)
}

/// Contract entry point for queries
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary> {
    contract::query(deps, env, msg)
}
