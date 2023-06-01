//! The service provider directory contract is for users to announce their service providers for
//! public use.

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]

pub use nym_service_provider_directory_common::error::{Result, SpContractError};

use nym_service_provider_directory_common::msg::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response};

mod constants;
mod contract;
mod state;

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod test_helpers;

/// Contract entry point for instantiation.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response> {
    contract::instantiate(deps, env, info, msg)
}

/// Contract entry point for migrations.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut<'_>, env: Env, msg: MigrateMsg) -> Result<Response, SpContractError> {
    contract::migrate(deps, env, msg)
}

/// Contract entry point for execution
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, SpContractError> {
    contract::execute(deps, env, info, msg)
}

/// Contract entry point for queries
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> Result<Binary> {
    contract::query(deps, env, msg)
}
